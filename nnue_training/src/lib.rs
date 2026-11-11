//! Configuration of the NNUE trainer: the fixed network layout the engine expects, the
//! command-line interface and the size check of the quantised output.
//!
//! Everything here is independent of bullet, so it is unit tested without a GPU.

/// Piece-square inputs per king bucket: 64 squares x 6 piece types x 2 colours.
pub const INPUT_SIZE: usize = 768;

/// King buckets on the half board (files a-d, rank 1 first); the board is mirrored when the
/// king stands on files e-h. Must match `src/nnue_service.rs`.
#[rustfmt::skip]
pub const BUCKET_LAYOUT: [usize; 32] = [
    0, 1, 2, 3,
    4, 4, 5, 5,
    6, 6, 6, 6,
    7, 7, 7, 7,
    8, 8, 8, 8,
    8, 8, 8, 8,
    9, 9, 9, 9,
    9, 9, 9, 9,
];

/// Number of distinct king buckets in [`BUCKET_LAYOUT`].
pub const INPUT_BUCKETS: usize = 10;

/// Output buckets by piece count, `(pieces - 2) / 4`.
pub const OUTPUT_BUCKETS: usize = 8;

/// Quantisation of the feature transformer (weights and biases).
pub const QA: i16 = 255;
/// Quantisation of the output weights.
pub const QB: i16 = 64;

/// Hidden neurons per perspective of the network the engine loads today.
pub const DEFAULT_HIDDEN: usize = 256;
/// Centipawns per sigmoid unit; every search margin of the engine is calibrated on 400.
pub const DEFAULT_EVAL_SCALE: f32 = 400.0;
/// Share of the game result in the training target.
pub const DEFAULT_WDL: f32 = 0.5;
/// Positions per batch.
pub const BATCH_SIZE: usize = 16_384;
/// Batches per superbatch: 6104 x 16384 is about 100 M positions.
pub const DEFAULT_BATCHES_PER_SUPERBATCH: usize = 6104;
/// Initial learning rate of the cosine schedule.
pub const INITIAL_LR: f32 = 0.001;
/// Factor from the initial to the final learning rate (0.3^5).
pub const FINAL_LR_FACTOR: f32 = 0.002_43; // 0.3^5
/// File sizes of quantised networks are padded to this many bytes.
pub const FILE_ALIGNMENT: usize = 64;

/// Number of king buckets used by a layout: the largest bucket index plus one.
pub fn bucket_count(layout: &[usize]) -> usize {
    layout.iter().max().map_or(0, |&max| max + 1)
}

/// Size in bytes of `quantised.bin` for a given hidden size: i16 values
/// `l0w l0b l1w l1b`, padded to [`FILE_ALIGNMENT`].
pub fn expected_quantised_bytes(hidden: usize) -> usize {
    let values = INPUT_BUCKETS * INPUT_SIZE * hidden // l0w (factoriser merged in)
        + hidden // l0b
        + OUTPUT_BUCKETS * 2 * hidden // l1w
        + OUTPUT_BUCKETS; // l1b
    (values * size_of::<i16>()).next_multiple_of(FILE_ALIGNMENT)
}

/// Everything one training run needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Shuffled bullet data files (binary `.data`); read in sequence, repeated as needed.
    pub data: Vec<String>,
    /// Last superbatch of the run.
    pub superbatches: usize,
    /// First superbatch (greater than 1 when resuming).
    pub start_superbatch: usize,
    pub batches_per_superbatch: usize,
    /// Checkpoint directory to resume from (weights and optimiser state).
    pub resume: Option<String>,
    /// Weights file to start from (weights only, fresh optimiser state).
    pub load_weights: Option<String>,
    /// Network id; checkpoints are written to `<output_dir>/<name>-<superbatch>/`.
    pub name: String,
    pub output_dir: String,
    pub wdl: f32,
    pub hidden: usize,
    pub eval_scale: f32,
    /// CPU threads of the data loader.
    pub threads: usize,
    /// Save a checkpoint every this many superbatches (and always at the end).
    pub save_rate: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            superbatches: 0,
            start_superbatch: 1,
            batches_per_superbatch: DEFAULT_BATCHES_PER_SUPERBATCH,
            resume: None,
            load_weights: None,
            name: "suprah".to_string(),
            output_dir: "checkpoints".to_string(),
            wdl: DEFAULT_WDL,
            hidden: DEFAULT_HIDDEN,
            eval_scale: DEFAULT_EVAL_SCALE,
            threads: 4,
            save_rate: 10,
        }
    }
}

impl Config {
    /// Directory of the checkpoint written after the last superbatch.
    pub fn final_checkpoint_dir(&self) -> String {
        format!("{}/{}-{}", self.output_dir, self.name, self.superbatches)
    }

    /// Learning rate at the end of the cosine schedule.
    pub fn final_lr(&self) -> f32 {
        INITIAL_LR * FINAL_LR_FACTOR
    }
}

/// Result of parsing the command line.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Train(Config),
    /// Print the trainer's evaluation of reference positions for a checkpoint; no training.
    Evaluate(EvalRequest),
    Help,
}

/// Evaluation of reference positions with a trained checkpoint (`--eval-fens`).
#[derive(Debug, Clone, PartialEq)]
pub struct EvalRequest {
    /// Checkpoint directory (`--resume`).
    pub checkpoint: String,
    /// File with one FEN per line; empty lines and lines starting with `#` are skipped.
    pub fens: String,
    pub hidden: usize,
    pub eval_scale: f32,
}

/// FENs of a reference file: trimmed, without empty lines and `#` comments.
pub fn reference_fens(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

/// One output line: the FEN and the score in centipawns from the side to move's point of view.
pub fn format_reference_line(fen: &str, raw_output: f32, eval_scale: f32) -> String {
    format!("{fen} | {:.2}", raw_output * eval_scale)
}

/// Parses the arguments after the program name. Unknown options, missing or malformed values
/// and inconsistent settings are errors; nothing is silently replaced by a default.
pub fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut cfg = Config::default();
    let mut eval_fens = None;
    let mut iter = args.iter();

    while let Some(option) = iter.next() {
        if matches!(option.as_str(), "-h" | "--help") {
            return Ok(Command::Help);
        }
        let value = iter
            .next()
            .ok_or_else(|| format!("option {option} needs a value"))?;
        match option.as_str() {
            "-d" | "--data" => cfg.data.push(value.clone()),
            "-s" | "--superbatches" => cfg.superbatches = number(option, value)?,
            "--start" => cfg.start_superbatch = number(option, value)?,
            "--batches-per-superbatch" => cfg.batches_per_superbatch = number(option, value)?,
            "--resume" => cfg.resume = Some(value.clone()),
            "--load-weights" => cfg.load_weights = Some(value.clone()),
            "-n" | "--name" => cfg.name = value.clone(),
            "-o" | "--output-dir" => cfg.output_dir = value.clone(),
            "--wdl" => cfg.wdl = number(option, value)?,
            "--hidden" => cfg.hidden = number(option, value)?,
            "--scale" => cfg.eval_scale = number(option, value)?,
            "-t" | "--threads" => cfg.threads = number(option, value)?,
            "--save-rate" => cfg.save_rate = number(option, value)?,
            "--eval-fens" => eval_fens = Some(value.clone()),
            _ => return Err(format!("unknown option {option}")),
        }
    }

    if let Some(fens) = eval_fens {
        return evaluation_request(cfg, fens).map(Command::Evaluate);
    }

    validate(&cfg)?;
    Ok(Command::Train(cfg))
}

fn evaluation_request(cfg: Config, fens: String) -> Result<EvalRequest, String> {
    let checkpoint = cfg
        .resume
        .ok_or("--eval-fens needs the checkpoint to evaluate (--resume)")?;
    if cfg.load_weights.is_some() {
        return Err(
            "--eval-fens takes the checkpoint from --resume, not --load-weights".to_string(),
        );
    }
    if cfg.hidden == 0 || cfg.eval_scale <= 0.0 {
        return Err("--hidden must be at least 1 and --scale positive".to_string());
    }
    Ok(EvalRequest {
        checkpoint,
        fens,
        hidden: cfg.hidden,
        eval_scale: cfg.eval_scale,
    })
}

fn number<T: std::str::FromStr>(option: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("option {option}: invalid value {value:?}"))
}

fn validate(cfg: &Config) -> Result<(), String> {
    if cfg.data.is_empty() {
        return Err("no training data given (--data)".to_string());
    }
    if cfg.superbatches == 0 {
        return Err("--superbatches must be at least 1".to_string());
    }
    if cfg.start_superbatch == 0 || cfg.start_superbatch > cfg.superbatches {
        return Err("--start must lie between 1 and --superbatches".to_string());
    }
    if !(0.0..=1.0).contains(&cfg.wdl) {
        return Err("--wdl must lie in [0, 1]".to_string());
    }
    if cfg.eval_scale <= 0.0 {
        return Err("--scale must be positive".to_string());
    }
    if cfg.hidden == 0 || cfg.batches_per_superbatch == 0 || cfg.threads == 0 || cfg.save_rate == 0
    {
        return Err(
            "--hidden, --batches-per-superbatch, --threads and --save-rate must be at least 1"
                .to_string(),
        );
    }
    if cfg.resume.is_some() && cfg.load_weights.is_some() {
        return Err("--resume and --load-weights exclude each other".to_string());
    }
    Ok(())
}

/// Command-line help.
pub const USAGE: &str = "\
Suprah NNUE trainer (bullet)

Usage: suprah-nnue-trainer --data <FILE> --superbatches <N> [OPTIONS]

Options:
  -d, --data <FILE>                 Shuffled bullet data file; repeat for several files
  -s, --superbatches <N>            Last superbatch of the run
      --start <N>                   First superbatch, > 1 when resuming [default: 1]
      --batches-per-superbatch <N>  Batches of 16384 positions per superbatch [default: 6104]
      --resume <DIR>                Checkpoint directory: weights and optimiser state
      --load-weights <FILE>         Weights only, fresh optimiser state
  -n, --name <NAME>                 Network id [default: suprah]
  -o, --output-dir <DIR>            Checkpoint directory, created if missing [default: checkpoints]
      --wdl <X>                     Share of the game result in the target [default: 0.5]
      --hidden <N>                  Hidden neurons per perspective [default: 256]
      --scale <X>                   Evaluation scale in centipawns [default: 400]
  -t, --threads <N>                 Data loader threads [default: 4]
      --save-rate <N>               Checkpoint every N superbatches [default: 10]
      --eval-fens <FILE>            Evaluate the FENs of FILE with the checkpoint given by
                                    --resume and print `fen | centipawns (side to move)`;
                                    no training (reference for the engine's NNUE tests)
  -h, --help                        Show this help

The engine loads networks with 256 hidden neurons and scale 400; other values need engine changes.
";

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn train(list: &[&str]) -> Config {
        match parse_args(&args(list)) {
            Ok(Command::Train(cfg)) => cfg,
            other => panic!("expected a training config, got {other:?}"),
        }
    }

    #[test]
    fn layout_has_ten_buckets() {
        assert_eq!(bucket_count(&BUCKET_LAYOUT), INPUT_BUCKETS);
    }

    #[test]
    fn quantised_size_of_the_current_network() {
        // Size of the network the engine embeds today (`eval_models/quantised.bin`).
        assert_eq!(expected_quantised_bytes(256), 3_940_928);
    }

    #[test]
    fn quantised_size_is_aligned_and_grows_with_width() {
        for hidden in [32, 256, 512, 768, 1024] {
            assert_eq!(expected_quantised_bytes(hidden) % FILE_ALIGNMENT, 0);
        }
        let raw_512 = 2
            * (INPUT_BUCKETS * INPUT_SIZE * 512 + 512 + OUTPUT_BUCKETS * 2 * 512 + OUTPUT_BUCKETS);
        assert!(expected_quantised_bytes(512) >= raw_512);
        assert!(expected_quantised_bytes(512) < raw_512 + FILE_ALIGNMENT);
    }

    #[test]
    fn minimal_arguments_use_the_n1_defaults() {
        let cfg = train(&["--data", "a.data", "--superbatches", "8"]);
        assert_eq!(cfg.data, vec!["a.data".to_string()]);
        assert_eq!(cfg.superbatches, 8);
        assert_eq!(cfg.start_superbatch, 1);
        assert_eq!(cfg.hidden, 256);
        assert_eq!(cfg.eval_scale, 400.0);
        assert_eq!(cfg.wdl, 0.5);
        assert_eq!(cfg.batches_per_superbatch, 6104);
    }

    #[test]
    fn all_options_are_parsed() {
        let cfg = train(&[
            "-d",
            "a.data",
            "--data",
            "b.data",
            "-s",
            "40",
            "--start",
            "11",
            "--resume",
            "ck/n1-10",
            "-n",
            "n1",
            "--output-dir",
            "nets",
            "--wdl",
            "0.3",
            "--hidden",
            "512",
            "--scale",
            "400",
            "-t",
            "6",
            "--save-rate",
            "5",
            "--batches-per-superbatch",
            "100",
        ]);
        assert_eq!(cfg.data, vec!["a.data".to_string(), "b.data".to_string()]);
        assert_eq!((cfg.superbatches, cfg.start_superbatch), (40, 11));
        assert_eq!(cfg.resume.as_deref(), Some("ck/n1-10"));
        assert_eq!((cfg.name.as_str(), cfg.output_dir.as_str()), ("n1", "nets"));
        assert_eq!(
            (cfg.wdl, cfg.hidden, cfg.threads, cfg.save_rate),
            (0.3, 512, 6, 5)
        );
        assert_eq!(cfg.batches_per_superbatch, 100);
        assert_eq!(cfg.final_checkpoint_dir(), "nets/n1-40");
    }

    #[test]
    fn eval_fens_needs_a_checkpoint_and_no_schedule() {
        let request = match parse_args(&args(&["--eval-fens", "ref.txt", "--resume", "ck/n1-8"])) {
            Ok(Command::Evaluate(request)) => request,
            other => panic!("expected an evaluation request, got {other:?}"),
        };
        assert_eq!(request.fens, "ref.txt");
        assert_eq!(request.checkpoint, "ck/n1-8");
        assert_eq!((request.hidden, request.eval_scale), (256, 400.0));
        assert!(parse_args(&args(&["--eval-fens", "ref.txt"])).is_err());
        assert!(
            parse_args(&args(&[
                "--eval-fens",
                "ref.txt",
                "--load-weights",
                "w.bin"
            ]))
            .is_err()
        );
    }

    #[test]
    fn reference_files_skip_comments_and_blank_lines() {
        let text =
            "# header\n\n  4k3/8/8/8/8/8/8/3QK3 w - - 0 1  \n# note\n8/8/8/8/8/8/8/K6k b - - 0 1\n";
        assert_eq!(
            reference_fens(text),
            vec![
                "4k3/8/8/8/8/8/8/3QK3 w - - 0 1",
                "8/8/8/8/8/8/8/K6k b - - 0 1"
            ]
        );
    }

    #[test]
    fn reference_lines_carry_centipawns_for_the_side_to_move() {
        assert_eq!(
            format_reference_line("8/8/8/8/8/8/8/K6k b - - 0 1", 0.25, 400.0),
            "8/8/8/8/8/8/8/K6k b - - 0 1 | 100.00"
        );
        assert_eq!(format_reference_line("x", -1.5, 400.0), "x | -600.00");
    }

    #[test]
    fn help_is_recognised() {
        assert_eq!(parse_args(&args(&["--help"])), Ok(Command::Help));
        assert_eq!(parse_args(&args(&["-h"])), Ok(Command::Help));
    }

    #[test]
    fn invalid_input_is_rejected() {
        let bad: [&[&str]; 10] = [
            &["--superbatches", "8"],                           // no data
            &["--data", "a.data"],                              // no superbatches
            &["--data", "a.data", "-s", "eight"],               // not a number
            &["--data", "a.data", "-s", "8", "--wdl", "1.5"],   // wdl outside [0, 1]
            &["--data", "a.data", "-s", "8", "--start", "9"],   // start after end
            &["--data", "a.data", "-s", "8", "--hidden", "0"],  // empty layer
            &["--data", "a.data", "-s", "8", "--unknown", "1"], // unknown option
            &["--data", "a.data", "-s"],                        // missing value
            &[
                "--data",
                "a.data",
                "-s",
                "8",
                "--resume",
                "x",
                "--load-weights",
                "y",
            ], // both
            &["--data", "a.data", "-s", "8", "--save-rate", "0"], // never saves
        ];
        for case in bad {
            assert!(parse_args(&args(case)).is_err(), "accepted {case:?}");
        }
    }

    #[test]
    fn final_learning_rate_matches_the_reference_schedule() {
        let expected = 0.001 * 0.3f32.powi(5);
        assert!((Config::default().final_lr() - expected).abs() < 1e-9);
    }
}
