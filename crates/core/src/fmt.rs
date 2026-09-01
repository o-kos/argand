//! Compact human-readable units for the terminal report.
//!
//! Ported from the sgvr CLI so that both tools print lengths and counts the
//! same way.

/// Sample counts as `430 spl`, `43.2 kspl`, `1.2 Mspl`, ...
pub fn format_samples(count: u64) -> String {
    const UNITS: [(f64, &str); 4] = [(1e3, "kspl"), (1e6, "Mspl"), (1e9, "Gspl"), (1e12, "Tspl")];

    if count < 1000 {
        return format!("{count} spl");
    }

    let value = count as f64;
    for (i, (scale, unit)) in UNITS.iter().enumerate() {
        let next = UNITS.get(i + 1).map(|(s, _)| *s).unwrap_or(f64::INFINITY);
        if value >= next {
            continue;
        }
        let scaled = round1(value / scale);
        // Rounding can push 999.96k up to 1000k; promote it instead.
        if scaled >= 1000.0 {
            let (_, bigger) = UNITS.get(i + 1).copied().unwrap_or((1.0, "Tspl"));
            return format!("1 {bigger}");
        }
        return format!("{} {unit}", trim1(scaled));
    }

    format!("{} Tspl", trim1(round1(value / 1e12)))
}

/// Durations as `250ms`, `12.5s`, `5m30`, `1h20m`, ...
pub fn format_duration(seconds: f64) -> String {
    if seconds.is_nan() {
        return "?".to_string();
    }
    if seconds < 0.0 {
        return format!("-{}", format_duration(-seconds));
    }

    let total_ms = (seconds * 1000.0).round() as u64;
    if total_ms < 1000 {
        return format!("{total_ms}ms");
    }

    let secs = total_ms / 1000;
    let millis = total_ms % 1000;
    let frac = if millis == 0 {
        String::new()
    } else {
        format!(".{millis:03}").trim_end_matches('0').to_string()
    };

    if secs < 60 {
        return format!("{secs}{frac}s");
    }
    if secs < 3600 {
        let (m, s) = (secs / 60, secs % 60);
        if s == 0 && frac.is_empty() {
            return format!("{m}m");
        }
        return format!("{m}m{s:02}{frac}");
    }

    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if m == 0 && s == 0 && frac.is_empty() {
        return format!("{h}h");
    }
    if s == 0 && frac.is_empty() {
        return format!("{h}h{m:02}m");
    }
    format!("{h}h{m:02}m{s:02}{frac}")
}

/// Frequencies as `24 kHz`, `12.579887 MHz`, `17.578 Hz`, ...
///
/// Decimals are chosen so every unit resolves to one hertz -- three for kHz,
/// six for MHz -- and trailing zeros are trimmed. Printing a fixed number of
/// decimals instead gives either useless precision (`10.886719 kHz`, a
/// millionth of a hertz) or not enough (`12.58 MHz`, ten kilohertz of slop).
pub fn format_hz(hz: f64) -> String {
    let abs = hz.abs();
    let (value, unit, decimals) = if abs >= 1e9 {
        (hz / 1e9, "GHz", 9)
    } else if abs >= 1e6 {
        (hz / 1e6, "MHz", 6)
    } else if abs >= 1e3 {
        (hz / 1e3, "kHz", 3)
    } else {
        (hz, "Hz", 3)
    };
    format!("{} {unit}", trim_decimals(value, decimals))
}

/// Byte counts as `248 KiB`, `1.4 MiB`, ...
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{} {}", trim1(round1(value)), UNITS[unit])
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn trim1(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

fn trim_decimals(value: f64, decimals: usize) -> String {
    let s = format!("{value:.decimals$}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    include!("fmt_tests.rs");
}
