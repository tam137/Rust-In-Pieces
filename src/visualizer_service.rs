use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;

/// Telemetry frame capturing board state, move details, and NNUE neuron activations.
#[derive(Clone, Debug, Serialize)]
pub struct NNUEMoveFrame {
    pub ply: usize,
    pub fen: String,
    pub move_uci: String,
    pub white_acc: Vec<i16>,
    pub black_acc: Vec<i16>,
    pub white_screlu: Vec<i32>,
    pub black_screlu: Vec<i32>,
    pub white_weights: Vec<i16>,
    pub black_weights: Vec<i16>,
    pub white_contrib: Vec<i32>,
    pub black_contrib: Vec<i32>,
    pub eval_cp: i16,
    pub white_king_bucket: usize,
    pub black_king_bucket: usize,
    pub output_bucket: usize,
    pub side_to_move: String,
}

pub struct VisualizerService;

impl VisualizerService {
    /// Generates a standalone, beautiful HTML/JS replayer dashboard from a series of NNUEMoveFrame.
    pub fn generate_html_report(frames: &[NNUEMoveFrame]) -> Result<String, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let filename = format!("reports/nnue_report_{}.html", timestamp);
        Self::generate_html_report_to_path(frames, &filename)
    }

    /// Generates or updates the HTML report at a specific target file path.
    pub fn generate_html_report_to_path(frames: &[NNUEMoveFrame], path_str: &str) -> Result<String, String> {
        let reports_dir = Path::new("reports");
        if !reports_dir.exists() {
            fs::create_dir_all(reports_dir).map_err(|e| format!("Failed to create reports directory: {}", e))?;
        }

        let filename = path_str.to_string();
        let frames_json = serde_json::to_string(frames).map_err(|e| format!("JSON serialization error: {}", e))?;

        let html_content = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Suprah NNUE Neuron Visualizer</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;600;700&family=JetBrains+Mono:wght@400;600&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg-base: #0B0E14;
            --bg-card: #151922;
            --bg-card-hover: #1C2230;
            --accent-cyan: #00F0FF;
            --accent-green: #00FF87;
            --accent-purple: #9D4EDD;
            --accent-red: #FF0055;
            --text-main: #F0F4F8;
            --text-muted: #8A99AD;
            --border-subtle: rgba(255, 255, 255, 0.08);
            --square-light: #E0E4EC;
            --square-dark: #4A5568;
            --square-highlight: rgba(0, 240, 255, 0.4);
        }}

        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        html, body {{
            height: 100vh;
            max-height: 100vh;
            overflow: hidden;
            font-family: 'Outfit', sans-serif;
            background-color: var(--bg-base);
            color: var(--text-main);
            display: flex;
            flex-direction: column;
            padding: 16px 24px;
        }}

        header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding-bottom: 12px;
            border-bottom: 1px solid var(--border-subtle);
            margin-bottom: 16px;
            flex-shrink: 0;
        }}

        h1 {{ font-size: 22px; font-weight: 700; background: linear-gradient(135deg, var(--accent-cyan), var(--accent-green)); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }}

        .dashboard {{ display: grid; grid-template-columns: 360px 1fr; gap: 20px; flex-grow: 1; min-height: 0; overflow: hidden; }}
        .panel {{ background: var(--bg-card); border-radius: 16px; border: 1px solid var(--border-subtle); padding: 16px; display: flex; flex-direction: column; gap: 12px; min-height: 0; overflow: hidden; }}

        .board-container {{ width: 310px; height: 310px; margin: 0 auto; display: grid; grid-template-columns: repeat(8, 1fr); border-radius: 8px; overflow: hidden; box-shadow: 0 8px 32px rgba(0,0,0,0.5); border: 2px solid var(--border-subtle); flex-shrink: 0; }}
        .square {{ width: 38.75px; height: 38.75px; display: flex; justify-content: center; align-items: center; font-size: 25px; position: relative; user-select: none; }}
        .square.light {{ background-color: var(--square-light); color: #222; }}
        .square.dark {{ background-color: var(--square-dark); color: #fff; }}

        .controls {{ display: flex; justify-content: center; gap: 8px; align-items: center; margin-top: 4px; flex-shrink: 0; }}
        .btn {{ background: var(--bg-card-hover); border: 1px solid var(--border-subtle); color: var(--text-main); padding: 6px 14px; border-radius: 8px; cursor: pointer; font-weight: 600; transition: all 0.2s ease; }}
        .btn:hover {{ background: var(--accent-cyan); color: #000; border-color: var(--accent-cyan); }}

        .slider-container {{ display: flex; align-items: center; gap: 12px; margin-top: 4px; flex-shrink: 0; }}
        .slider {{ flex-grow: 1; accent-color: var(--accent-cyan); cursor: pointer; }}

        .meta-grid {{ display: grid; grid-template-columns: repeat(2, 1fr); gap: 10px; flex-shrink: 0; }}
        .meta-card {{ background: var(--bg-base); padding: 10px; border-radius: 8px; border: 1px solid var(--border-subtle); text-align: center; }}
        .meta-label {{ font-size: 11px; color: var(--text-muted); text-transform: uppercase; margin-bottom: 2px; letter-spacing: 0.5px; }}
        .meta-value {{ font-size: 16px; font-weight: 700; font-family: 'JetBrains Mono', monospace; }}

        .tab-bar {{ display: flex; gap: 12px; border-bottom: 1px solid var(--border-subtle); padding-bottom: 8px; flex-shrink: 0; }}
        .tab-btn {{ background: none; border: none; color: var(--text-muted); font-size: 14px; font-weight: 600; cursor: pointer; padding: 6px 12px; border-radius: 6px; transition: all 0.2s; }}
        .tab-btn.active {{ color: var(--accent-cyan); background: rgba(0, 240, 255, 0.1); border: 1px solid rgba(0,240,255,0.3); }}

        .neuron-section {{ flex-grow: 1; min-height: 0; display: flex; align-items: center; justify-content: center; overflow: hidden; padding: 4px; }}
        .neuron-grid {{ display: grid; grid-template-columns: repeat(16, 1fr); grid-template-rows: repeat(16, 1fr); gap: 3px; background: var(--bg-base); padding: 10px; border-radius: 12px; border: 1px solid var(--border-subtle); height: 100%; max-height: 100%; aspect-ratio: 1; box-sizing: border-box; }}
        .neuron-cell {{ width: 100%; height: 100%; min-width: 0; min-height: 0; border-radius: 2px; cursor: pointer; transition: transform 0.1s, box-shadow 0.1s; position: relative; }}
        .neuron-cell:hover {{ transform: scale(1.35); z-index: 10; box-shadow: 0 0 14px rgba(0,240,255,0.9); }}

        .tooltip {{ position: fixed; background: #0B0E14; color: #FFF; padding: 10px 14px; border-radius: 8px; font-size: 12px; font-family: 'JetBrains Mono', monospace; pointer-events: none; border: 1px solid var(--accent-cyan); display: none; z-index: 100; box-shadow: 0 8px 24px rgba(0,0,0,0.8); }}
    </style>
</head>
<body>

<header>
    <div>
        <h1>Suprah NNUE Neuron Visualizer</h1>
        <p style="color: var(--text-muted); font-size: 13px; margin-top: 2px;">Interactive Replayer & Dual-Perspective Accumulator Heatmaps</p>
    </div>
</header>

<div class="dashboard">
    <!-- Left Column: Chessboard & Controls -->
    <div class="panel">
        <div class="meta-grid">
            <div class="meta-card">
                <div class="meta-label">Evaluation</div>
                <div class="meta-value" id="eval-display">0.00</div>
            </div>
            <div class="meta-card">
                <div class="meta-label">Ply / Turn</div>
                <div class="meta-value" id="ply-display">0 / 0</div>
            </div>
        </div>

        <div class="board-container" id="board"></div>

        <div class="controls">
            <button class="btn" id="btn-first">&laquo;</button>
            <button class="btn" id="btn-prev">&lsaquo;</button>
            <button class="btn" id="btn-play">&#9654;</button>
            <button class="btn" id="btn-next">&rsaquo;</button>
            <button class="btn" id="btn-last">&raquo;</button>
        </div>

        <div class="slider-container">
            <span style="font-size: 12px; color: var(--text-muted);">0</span>
            <input type="range" id="move-slider" class="slider" min="0" max="0" value="0">
            <span style="font-size: 12px; color: var(--text-muted);" id="slider-max">0</span>
        </div>

        <div class="meta-grid" style="margin-top: 8px;">
            <div class="meta-card">
                <div class="meta-label">King Buckets (W / B)</div>
                <div class="meta-value" id="king-buckets">0 / 0</div>
            </div>
            <div class="meta-card">
                <div class="meta-label">Output Bucket</div>
                <div class="meta-value" id="output-bucket">0</div>
            </div>
        </div>
    </div>

    <!-- Right Column: Neurons Heatmap Grid -->
    <div class="panel">
        <div style="display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--border-subtle); padding-bottom: 8px; flex-shrink: 0;">
            <div class="tab-bar" style="border-bottom: none; padding-bottom: 0;">
                <button class="tab-btn active" id="tab-white">White Perspective</button>
                <button class="tab-btn" id="tab-black">Black Perspective</button>
            </div>
            <div class="tab-bar" style="border-bottom: none; padding-bottom: 0;">
                <button class="tab-btn active" id="mode-screlu">Layer 1 (SCReLU)</button>
                <button class="tab-btn" id="mode-contrib">Layer 2 (Weighted Contrib)</button>
            </div>
        </div>

        <div class="meta-grid" style="margin-bottom: 4px;">
            <div class="meta-card">
                <div class="meta-label">Weighted Sum (W + B)</div>
                <div class="meta-value" id="weighted-sum">0</div>
            </div>
            <div class="meta-card">
                <div class="meta-label">Pos / Neg Contrib (Perspective)</div>
                <div class="meta-value" id="pos-neg-contrib">+0 / 0</div>
            </div>
        </div>

        <div class="neuron-section">
            <div class="neuron-grid" id="neuron-grid"></div>
        </div>
    </div>
</div>

<div class="tooltip" id="tooltip"></div>

<script>
    const frames = {};
    let currentFrameIdx = 0;
    let currentTab = 'white';
    let currentMode = 'screlu'; // 'screlu' or 'contrib'
    let isPlaying = false;
    let playInterval = null;

    const pieceSymbols = {{
        'P': '♙', 'N': '♘', 'B': '♗', 'R': '♖', 'Q': '♕', 'K': '♔',
        'p': '♟', 'n': '♞', 'b': '♝', 'r': '♜', 'q': '♛', 'k': '♚'
    }};

    function parseFEN(fen) {{
        const board = Array(64).fill(null);
        const parts = fen.split(' ');
        const rows = parts[0].split('/');
        let sq = 0;
        for (let r = 0; r < 8; r++) {{
            for (let c = 0; c < rows[r].length; c++) {{
                const char = rows[r][c];
                if (/\d/.test(char)) {{
                    sq += parseInt(char, 10);
                }} else {{
                    board[sq] = char;
                    sq++;
                }}
            }}
        }}
        return board;
    }}

    function renderBoard(fen) {{
        const boardEl = document.getElementById('board');
        boardEl.innerHTML = '';
        const squares = parseFEN(fen);
        for (let i = 0; i < 64; i++) {{
            const row = Math.floor(i / 8);
            const col = i % 8;
            const isLight = (row + col) % 2 === 0;
            const squareEl = document.createElement('div');
            squareEl.className = `square ${{isLight ? 'light' : 'dark'}}`;
            if (squares[i]) {{
                squareEl.textContent = pieceSymbols[squares[i]] || squares[i];
            }}
            boardEl.appendChild(squareEl);
        }}
    }}

    function getNeuronColor(accVal, screluVal, weightVal, contribVal) {{
        if (currentMode === 'screlu') {{
            if (screluVal === 0) return 'rgba(25, 30, 45, 0.7)';
            const normalized = Math.min(screluVal / 65025, 1.0);
            const cyan = Math.floor(normalized * 255);
            const green = Math.floor(normalized * 220);
            return `rgba(0, ${{cyan}}, ${{green}}, ${{0.4 + normalized * 0.6}})`;
        }} else {{
            if (contribVal === 0) return 'rgba(25, 30, 45, 0.7)';
            if (contribVal > 0) {{
                const norm = Math.min(contribVal / 50000, 1.0);
                const g = Math.floor(150 + norm * 105);
                return `rgba(0, ${{g}}, 135, ${{0.4 + norm * 0.6}})`;
            }} else {{
                const norm = Math.min(Math.abs(contribVal) / 50000, 1.0);
                const r = Math.floor(180 + norm * 75);
                return `rgba(${{r}}, 20, 85, ${{0.4 + norm * 0.6}})`;
            }}
        }}
    }}

    function renderNeurons(frame) {{
        const gridEl = document.getElementById('neuron-grid');
        gridEl.innerHTML = '';

        const acc = currentTab === 'white' ? frame.white_acc : frame.black_acc;
        const screlu = currentTab === 'white' ? frame.white_screlu : frame.black_screlu;
        const weights = currentTab === 'white' ? frame.white_weights : frame.black_weights;
        const contrib = currentTab === 'white' ? frame.white_contrib : frame.black_contrib;

        let posSum = 0;
        let negSum = 0;
        for (let i = 0; i < 256; i++) {{
            if (contrib[i] > 0) posSum += contrib[i];
            else negSum += contrib[i];
        }}

        let totalWhiteContrib = frame.white_contrib.reduce((a, b) => a + b, 0);
        let totalBlackContrib = frame.black_contrib.reduce((a, b) => a + b, 0);
        let totalWeightedSum = totalWhiteContrib + totalBlackContrib;

        document.getElementById('weighted-sum').textContent = totalWeightedSum.toLocaleString();
        document.getElementById('pos-neg-contrib').textContent = `+${{posSum.toLocaleString()}} / ${{negSum.toLocaleString()}}`;

        for (let i = 0; i < 256; i++) {{
            const cell = document.createElement('div');
            cell.className = 'neuron-cell';
            cell.style.backgroundColor = getNeuronColor(acc[i], screlu[i], weights[i], contrib[i]);
            
            cell.addEventListener('mouseenter', (e) => {{
                const tooltip = document.getElementById('tooltip');
                tooltip.style.display = 'block';
                tooltip.style.left = (e.clientX + 15) + 'px';
                tooltip.style.top = (e.clientY + 15) + 'px';
                tooltip.innerHTML = `<strong>Neuron #${{i}} (${{currentTab.toUpperCase()}})</strong><br>` +
                    `Raw Acc: ${{acc[i]}}<br>` +
                    `SCReLU Activation: ${{screlu[i]}}<br>` +
                    `Output Weight: ${{weights[i]}}<br>` +
                    `<strong>Weighted Contrib: ${{contrib[i].toLocaleString()}}</strong>`;
            }});

            cell.addEventListener('mouseleave', () => {{
                document.getElementById('tooltip').style.display = 'none';
            }});

            gridEl.appendChild(cell);
        }}
    }}

    function updateFrame(idx) {{
        if (!frames || frames.length === 0) return;
        currentFrameIdx = Math.max(0, Math.min(idx, frames.length - 1));
        const frame = frames[currentFrameIdx];

        renderBoard(frame.fen);
        renderNeurons(frame);

        document.getElementById('eval-display').textContent = (frame.eval_cp / 100).toFixed(2);
        document.getElementById('ply-display').textContent = `${{frame.ply}} (${{frame.move_uci || 'Start'}})`;
        document.getElementById('move-slider').value = currentFrameIdx;
        document.getElementById('king-buckets').textContent = `${{frame.white_king_bucket}} / ${{frame.black_king_bucket}}`;
        document.getElementById('output-bucket').textContent = frame.output_bucket;
    }}

    function init() {{
        if (!frames || frames.length === 0) return;
        const slider = document.getElementById('move-slider');
        slider.max = frames.length - 1;
        document.getElementById('slider-max').textContent = frames.length - 1;

        document.getElementById('btn-first').addEventListener('click', () => updateFrame(0));
        document.getElementById('btn-prev').addEventListener('click', () => updateFrame(currentFrameIdx - 1));
        document.getElementById('btn-next').addEventListener('click', () => updateFrame(currentFrameIdx + 1));
        document.getElementById('btn-last').addEventListener('click', () => updateFrame(frames.length - 1));
        slider.addEventListener('input', (e) => updateFrame(parseInt(e.target.value, 10)));

        document.getElementById('tab-white').addEventListener('click', () => {{
            currentTab = 'white';
            document.getElementById('tab-white').classList.add('active');
            document.getElementById('tab-black').classList.remove('active');
            updateFrame(currentFrameIdx);
        }});

        document.getElementById('tab-black').addEventListener('click', () => {{
            currentTab = 'black';
            document.getElementById('tab-black').classList.add('active');
            document.getElementById('tab-white').classList.remove('active');
            updateFrame(currentFrameIdx);
        }});

        document.getElementById('mode-screlu').addEventListener('click', () => {{
            currentMode = 'screlu';
            document.getElementById('mode-screlu').classList.add('active');
            document.getElementById('mode-contrib').classList.remove('active');
            updateFrame(currentFrameIdx);
        }});

        document.getElementById('mode-contrib').addEventListener('click', () => {{
            currentMode = 'contrib';
            document.getElementById('mode-contrib').classList.add('active');
            document.getElementById('mode-screlu').classList.remove('active');
            updateFrame(currentFrameIdx);
        }});

        document.getElementById('btn-play').addEventListener('click', () => {{
            isPlaying = !isPlaying;
            document.getElementById('btn-play').innerHTML = isPlaying ? '&#10074;&#10074;' : '&#9654;';
            if (isPlaying) {{
                playInterval = setInterval(() => {{
                    if (currentFrameIdx >= frames.length - 1) {{
                        currentFrameIdx = 0;
                    }}
                    updateFrame(currentFrameIdx + 1);
                }}, 800);
            }} else {{
                clearInterval(playInterval);
            }}
        }});

        updateFrame(0);
    }}

    window.onload = init;
</script>
</body>
</html>"#,
            frames_json
        );

        let mut file = File::create(&filename).map_err(|e| format!("Failed to create HTML report file: {}", e))?;
        file.write_all(html_content.as_bytes()).map_err(|e| format!("Failed to write HTML content: {}", e))?;

        Ok(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_html_report() {
        let frame = NNUEMoveFrame {
            ply: 0,
            fen: crate::model::INIT_BOARD_FEN.to_string(),
            move_uci: "e2e4".to_string(),
            white_acc: vec![10; 256],
            black_acc: vec![5; 256],
            white_screlu: vec![100; 256],
            black_screlu: vec![25; 256],
            white_weights: vec![1; 256],
            black_weights: vec![-1; 256],
            white_contrib: vec![100; 256],
            black_contrib: vec![-25; 256],
            eval_cp: 25,
            white_king_bucket: 0,
            black_king_bucket: 0,
            output_bucket: 7,
            side_to_move: "w".to_string(),
        };

        let result = VisualizerService::generate_html_report(&[frame]);
        assert!(result.is_ok(), "HTML report generation failed: {:?}", result.err());
        let path = result.unwrap();
        assert!(Path::new(&path).exists(), "Report file was not created: {}", path);
    }
}
