//! Trains a Suprah NNUE network with bullet.
//!
//! Architecture (N1, the layout the engine loads):
//! `(768 x 10 king buckets, horizontally mirrored -> HIDDEN) x 2 -> 1 x 8 output buckets`,
//! SCReLU, with a 768 x HIDDEN factoriser that is merged into every king bucket on save.
//! Quantisation and file layout match `src/nnue_service.rs`.

#[cfg(not(any(feature = "hip", feature = "cpu")))]
compile_error!("select a backend: --features hip (AMD GPU) or --features cpu (tests only)");

use bullet::{
    game::{
        inputs::{ChessBucketsMirrored, get_num_buckets},
        outputs::MaterialCount,
    },
    nn::{
        InitSettings, Shape,
        optimiser::{AdamW, AdamWParams},
    },
    trainer::{
        save::SavedFormat,
        schedule::{TrainingSchedule, TrainingSteps, lr, wdl},
        settings::LocalSettings,
    },
    value::{ValueTrainerBuilder, loader::DirectSequentialDataLoader},
};
use suprah_nnue_trainer::{
    BATCH_SIZE, BUCKET_LAYOUT, Command, Config, INITIAL_LR, INPUT_BUCKETS, INPUT_SIZE, OUTPUT_BUCKETS, QA, QB,
    USAGE, expected_quantised_bytes, parse_args,
};

/// Bound of the feature-transformer weights; tighter than bullet's default because the
/// factoriser is added to every bucket.
const FT_WEIGHT_CLIP: f32 = 0.99;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = match parse_args(&args) {
        Ok(Command::Train(cfg)) => cfg,
        Ok(Command::Help) => {
            print!("{USAGE}");
            return;
        }
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            std::process::exit(2);
        }
    };

    print_config(&cfg);
    if let Err(e) = std::fs::create_dir_all(&cfg.output_dir) {
        eprintln!("error: cannot create {}: {e}", cfg.output_dir);
        std::process::exit(1);
    }

    train(&cfg);

    if let Err(message) = check_final_net(&cfg) {
        eprintln!("error: {message}");
        std::process::exit(1);
    }
}

fn train(cfg: &Config) {
    const NUM_INPUT_BUCKETS: usize = get_num_buckets(&BUCKET_LAYOUT);
    const _: () = assert!(NUM_INPUT_BUCKETS == INPUT_BUCKETS);
    let hidden = cfg.hidden;

    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(AdamW)
        .inputs(ChessBucketsMirrored::new(BUCKET_LAYOUT))
        .output_buckets(MaterialCount::<OUTPUT_BUCKETS>)
        .save_format(&[
            SavedFormat::id("l0w")
                .transform(|store, weights| {
                    let factoriser = store.get("l0f").values.repeat(NUM_INPUT_BUCKETS);
                    weights.into_iter().zip(factoriser).map(|(w, f)| w + f).collect()
                })
                .round()
                .quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            SavedFormat::id("l1w").round().quantise::<i16>(QB).transpose(),
            SavedFormat::id("l1b").round().quantise::<i16>(QA * QB),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm_inputs, ntm_inputs, output_buckets| {
            let l0f = builder.new_weights("l0f", Shape::new(hidden, INPUT_SIZE), InitSettings::Zeroed);
            let expanded_factoriser = l0f.repeat(NUM_INPUT_BUCKETS);

            let mut l0 = builder.new_affine("l0", INPUT_SIZE * NUM_INPUT_BUCKETS, hidden);
            l0.weights = l0.weights + expanded_factoriser;

            let l1 = builder.new_affine("l1", 2 * hidden, OUTPUT_BUCKETS);

            let stm_hidden = l0.forward(stm_inputs).screlu();
            let ntm_hidden = l0.forward(ntm_inputs).screlu();
            l1.forward(stm_hidden.concat(ntm_hidden)).select(output_buckets)
        });

    let clip = AdamWParams { max_weight: FT_WEIGHT_CLIP, min_weight: -FT_WEIGHT_CLIP, ..Default::default() };
    trainer.optimiser.set_params_for_weight("l0w", clip);
    trainer.optimiser.set_params_for_weight("l0f", clip);

    if let Some(dir) = &cfg.resume {
        trainer.load_from_checkpoint(dir);
    }
    if let Some(file) = &cfg.load_weights {
        if let Err(e) = trainer.optimiser.load_weights_from_file(file) {
            eprintln!("error: cannot load weights from {file}: {e:?}");
            std::process::exit(1);
        }
    }

    let schedule = TrainingSchedule {
        net_id: cfg.name.clone(),
        eval_scale: cfg.eval_scale,
        steps: TrainingSteps {
            batch_size: BATCH_SIZE,
            batches_per_superbatch: cfg.batches_per_superbatch,
            start_superbatch: cfg.start_superbatch,
            end_superbatch: cfg.superbatches,
        },
        wdl_scheduler: wdl::ConstantWDL { value: cfg.wdl },
        lr_scheduler: lr::CosineDecayLR {
            initial_lr: INITIAL_LR,
            final_lr: cfg.final_lr(),
            final_superbatch: cfg.superbatches,
        },
        save_rate: cfg.save_rate,
    };

    let settings = LocalSettings {
        threads: cfg.threads,
        test_set: None,
        output_directory: &cfg.output_dir,
        batch_queue_size: 32,
    };

    let data: Vec<&str> = cfg.data.iter().map(String::as_str).collect();
    let loader = DirectSequentialDataLoader::new(&data);

    trainer.run(&schedule, &settings, &loader);
}

/// The quantised file of the last checkpoint must have exactly the size the engine expects.
fn check_final_net(cfg: &Config) -> Result<(), String> {
    let path = format!("{}/quantised.bin", cfg.final_checkpoint_dir());
    let actual = std::fs::metadata(&path).map_err(|e| format!("{path}: {e}"))?.len() as usize;
    let expected = expected_quantised_bytes(cfg.hidden);
    if actual != expected {
        return Err(format!("{path}: {actual} bytes, expected {expected}"));
    }
    println!("Final network {path}: {actual} bytes, size as expected.");
    Ok(())
}

fn print_config(cfg: &Config) {
    println!("=== Suprah NNUE trainer ===");
    println!("Data            : {}", cfg.data.join(", "));
    println!("Superbatches    : {} to {} ({} batches of {BATCH_SIZE})", cfg.start_superbatch, cfg.superbatches, cfg.batches_per_superbatch);
    println!("Layout          : (768x{INPUT_BUCKETS} mirrored -> {})x2 -> 1x{OUTPUT_BUCKETS}, SCReLU", cfg.hidden);
    println!("WDL / scale     : {} / {}", cfg.wdl, cfg.eval_scale);
    println!("Learning rate   : cosine {INITIAL_LR} -> {:e}", cfg.final_lr());
    println!("Output          : {}/{}-<superbatch>/ every {} superbatches", cfg.output_dir, cfg.name, cfg.save_rate);
    if let Some(dir) = &cfg.resume {
        println!("Resume from     : {dir}");
    }
    if let Some(file) = &cfg.load_weights {
        println!("Load weights    : {file}");
    }
    println!();
}
