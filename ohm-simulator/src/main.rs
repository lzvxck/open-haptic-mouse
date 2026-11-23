use ohm_core::{Event, Profile};
use ohm_simulator::{replay, run_device, Scenario, SimHapticChain};

const LATENCY_RAPID_WINDOW_US: u32 = 20_000;

fn event_name(ev: Event) -> &'static str {
    match ev {
        Event::Down => "DOWN",
        Event::RapidDown => "RAPID_DOWN",
        Event::Up => "UP",
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let latency = args.first().is_some_and(|a| a == "--latency");
    if latency {
        args.remove(0);
    }
    let path = args.first().cloned().unwrap_or_else(|| {
        eprintln!("uso: ohm-simulator [--latency] <scenario.json>");
        std::process::exit(2);
    });
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("no se pudo leer {path}: {e}");
        std::process::exit(1);
    });
    let scenario = Scenario::from_json(&json).unwrap_or_else(|e| {
        eprintln!("escenario inválido {path}: {e}");
        std::process::exit(1);
    });
    if !latency {
        for (t, ev) in replay(scenario, Profile::default()) {
            println!("t={}us {}", t.0, event_name(ev));
        }
        return;
    }
    let profile = Profile { rapid_trigger_window_us: LATENCY_RAPID_WINDOW_US, ..Profile::default() };
    let (outputs, chain) = run_device(scenario, profile, SimHapticChain::default());
    for out in outputs {
        let Some(ev) = out.event else { continue };
        match out.detect_to_trigger_us {
            Some(us) => println!("t={}us {} detect_to_trigger_us={us}", out.sample.t.0, event_name(ev)),
            None => println!("t={}us {}", out.sample.t.0, event_name(ev)),
        }
    }
    println!(
        "MODELLED, not measured on hardware: trigger_delay_us={} is an unverified DRV2605L datasheet assumption.",
        chain.trigger_delay_us
    );
}
