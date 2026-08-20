// Hide console window in release builds so this can run resident in the background.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod single_instance;
mod tray;

use serde_json::json;
use std::f64::consts::PI;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};
use tiny_http::{Response, Server};

const CHROMA_BASE: &str = "http://localhost:54235/razer/chromasdk";
const LISTEN_ADDR: &str = "127.0.0.1:8765";
const KB_ROWS: usize = 6;
const KB_COLS: usize = 22;

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    // A small, capped connection pool: only ever one Chroma session URI is
    // live at a time, so there's nothing to gain from a bigger pool.
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .max_idle_connections(2)
            .max_idle_connections_per_host(2)
            .build()
    })
}

fn register_session() -> Option<String> {
    let body = json!({
        "title": "ClaudeMoodLight",
        "description": "Claude Code generation status light",
        "author": { "name": "local", "contact": "local@localhost" },
        "device_supported": ["keyboard"],
        "category": "application"
    });

    match agent().post(CHROMA_BASE).send_json(body) {
        Ok(resp) => resp
            .into_json::<serde_json::Value>()
            .ok()
            .and_then(|v| v.get("uri").and_then(|u| u.as_str()).map(|s| s.to_string())),
        Err(e) => {
            eprintln!("[register] request error: {e}");
            None
        }
    }
}

fn ensure_session(uri: &Arc<Mutex<Option<String>>>) -> Option<String> {
    {
        let guard = uri.lock().unwrap();
        if let Some(u) = guard.as_ref() {
            return Some(u.clone());
        }
    }
    let new_uri = register_session();
    if let Some(u) = &new_uri {
        *uri.lock().unwrap() = Some(u.clone());
    }
    new_uri
}

/// Scale each RGB channel of a packed 0x00BBGGRR color by `factor` (0.0-1.0).
fn scale_color(color: u32, factor: f64) -> u32 {
    let r = (color & 0xFF) as f64;
    let g = ((color >> 8) & 0xFF) as f64;
    let b = ((color >> 16) & 0xFF) as f64;
    let r = (r * factor).round() as u32 & 0xFF;
    let g = (g * factor).round() as u32 & 0xFF;
    let b = (b * factor).round() as u32 & 0xFF;
    r | (g << 8) | (b << 16)
}

fn put_keyboard_color(uri: &Arc<Mutex<Option<String>>>, color: u32) {
    let Some(u) = ensure_session(uri) else { return };
    let grid: Vec<Vec<u32>> = (0..KB_ROWS).map(|_| vec![color; KB_COLS]).collect();
    let body = json!({ "effect": "CHROMA_CUSTOM", "param": grid });
    let url = format!("{u}/keyboard");
    if agent().put(&url).send_json(body).is_err() {
        // session likely expired, drop it so the next call re-registers
        *uri.lock().unwrap() = None;
    }
}

struct BreathParams {
    period: Duration,
    min: f64,
    step: Duration,
}

/// Continuously re-applies the current base color with a sine-wave brightness
/// envelope so the keyboard appears to "breathe" instead of showing a flat
/// color. Runs at all times; also doubles as the keep-alive heartbeat since
/// it's well under the SDK's 15s session timeout. Identical consecutive
/// colors (common near the top/bottom of the wave once rounded to 8-bit
/// channels) are not re-sent, since Chroma already shows them and skipping
/// the call is the single biggest resource saving available here.
fn breathing_loop(uri: Arc<Mutex<Option<String>>>, base_color: Arc<AtomicU32>, params: BreathParams) {
    let start = Instant::now();
    let mut last_sent: Option<u32> = None;
    loop {
        let elapsed = start.elapsed().as_secs_f64();
        let phase = (elapsed % params.period.as_secs_f64()) / params.period.as_secs_f64();
        let brightness = params.min + (1.0 - params.min) * (0.5 - 0.5 * (2.0 * PI * phase).cos());
        let color = scale_color(base_color.load(Ordering::Relaxed), brightness);
        if last_sent != Some(color) {
            put_keyboard_color(&uri, color);
            last_sent = Some(color);
        }
        thread::sleep(params.step);
    }
}

fn main() {
    if !single_instance::acquire() {
        // Another copy is already running; this is just a periodic
        // Task Scheduler launch attempt used to auto-heal after a crash.
        return;
    }

    let cfg = config::load_or_create();
    let color_generating = config::parse_color(&cfg.color_generating);
    let color_idle = config::parse_color(&cfg.color_idle);
    let color_waiting = config::parse_color(&cfg.color_waiting);
    let color_compacting = config::parse_color(&cfg.color_compacting);

    let uri: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let base_color = Arc::new(AtomicU32::new(color_idle));

    {
        let uri = Arc::clone(&uri);
        let base_color = Arc::clone(&base_color);
        let params = BreathParams {
            period: Duration::from_millis(cfg.breath_period_ms),
            min: cfg.breath_min,
            step: Duration::from_millis(cfg.breath_step_ms),
        };
        thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(move || breathing_loop(uri, base_color, params))
            .expect("failed to spawn breathing thread");
    }

    thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| tray::run("Claude Code キーボードライト"))
        .expect("failed to spawn tray thread");

    let server = Server::http(LISTEN_ADDR).expect("failed to bind local port");
    for request in server.incoming_requests() {
        let color = match request.url() {
            "/generating" => Some(color_generating),
            "/idle" => Some(color_idle),
            "/waiting" => Some(color_waiting),
            "/compacting" => Some(color_compacting),
            _ => None,
        };

        if let Some(color) = color {
            base_color.store(color, Ordering::Relaxed);
            let _ = request.respond(Response::from_string("ok"));
        } else {
            let _ = request.respond(Response::from_string("unknown").with_status_code(404));
        }
    }
}
