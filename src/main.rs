use std::io::{self, Read, Write};
use std::process::Command;

const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[32m";
const WHITE: &str = "\x1b[97m";
const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";

fn get_quota_color(pct: u32) -> &'static str {
    if pct <= 10 {
        RED
    } else if pct <= 30 {
        YELLOW
    } else {
        GREEN
    }
}

fn get_context_color(pct: u32) -> &'static str {
    if pct >= 90 { RED } else { YELLOW }
}

fn get_git_branch(cwd: &str) -> String {
    let mut cmd = Command::new("git");
    cmd.args(["branch", "--show-current"]);
    if !cwd.is_empty() {
        cmd.current_dir(cwd);
    }
    match cmd.output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "none".to_string(),
    }
}

fn extract_string_field(json: &str, field: &str) -> Option<String> {
    let search = format!("\"{}\":", field);
    if let Some(idx) = json.find(&search) {
        let remainder = json[idx + search.len()..].trim_start();
        if let Some(stripped) = remainder.strip_prefix('"')
            && let Some(end) = stripped.find('"')
        {
            return Some(stripped[..end].to_string());
        }
    }
    None
}

fn extract_f64_field(json: &str, field: &str) -> Option<f64> {
    let search = format!("\"{}\":", field);
    if let Some(idx) = json.find(&search) {
        let remainder = &json[idx + search.len()..];
        let remainder = remainder.trim_start();
        let end = remainder
            .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
            .unwrap_or(remainder.len());
        if let Ok(val) = remainder[..end].parse::<f64>() {
            return Some(val);
        }
    }
    None
}

fn build_progress_bar(pct: u32, total_blocks: usize) -> String {
    let safe_pct = if pct > 100 { 100 } else { pct };
    let filled = ((safe_pct as f32 / 100.0) * (total_blocks as f32)).round() as usize;
    let empty = total_blocks.saturating_sub(filled);

    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn extract_quota_frac(json: &str, key: &str) -> Option<f64> {
    if let Some(quota_idx) = json.find("\"quota\":") {
        let q_block = &json[quota_idx..];
        let search = format!("\"{}\":", key);
        if let Some(k_idx) = q_block.find(&search)
            && let Some(frac_idx) = q_block[k_idx..].find("\"remaining_fraction\":")
        {
            let frac_str = q_block[k_idx + frac_idx + 21..].trim_start();
            let end_frac = frac_str
                .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                .unwrap_or(frac_str.len());
            if let Ok(val) = frac_str[..end_frac].parse::<f64>() {
                return Some(val);
            }
        }
    }
    None
}

fn extract_quota_reset_secs(json: &str, key: &str) -> Option<u32> {
    if let Some(quota_idx) = json.find("\"quota\":") {
        let q_block = &json[quota_idx..];
        let search = format!("\"{}\":", key);
        if let Some(k_idx) = q_block.find(&search)
            && let Some(reset_idx) = q_block[k_idx..].find("\"reset_in_seconds\":")
        {
            let reset_str = q_block[k_idx + reset_idx + 19..].trim_start();
            let end = reset_str
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(reset_str.len());
            if let Ok(val) = reset_str[..end].parse::<u32>() {
                return Some(val);
            }
        }
    }
    None
}

fn format_time_left(secs: u32) -> String {
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    if hours > 48 {
        let days = hours / 24;
        format!("{}d", days)
    } else if hours > 24 {
        let days = hours / 24;
        let rem_hours = hours % 24;
        format!("{}d{}h", days, rem_hours)
    } else if hours > 0 {
        format!("{}h{}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_ansi = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_ansi = true;
        } else if in_ansi {
            if c.is_ascii_alphabetic() {
                in_ansi = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let term_width = extract_f64_field(&input, "terminal_width").unwrap_or(0.0) as usize;

    let cwd = extract_string_field(&input, "cwd").unwrap_or_default();
    let cwd_basename = cwd.rsplit('/').next().unwrap_or(&cwd);
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/ash".to_string());

    let branch_json = extract_string_field(&input, "branch").unwrap_or_default();
    let branch_raw = if branch_json.is_empty() || branch_json == "none" {
        get_git_branch(&cwd)
    } else {
        branch_json
    };

    let model = extract_string_field(&input, "display_name")
        .or_else(|| extract_string_field(&input, "id"))
        .unwrap_or_else(|| "Unknown Model".to_string());

    let context_frac = extract_f64_field(&input, "used_percentage").unwrap_or(0.0);
    let context_pct = context_frac.round() as u32;

    let active_agents =
        extract_string_field(&input, "agent_state").unwrap_or_else(|| "working".to_string());

    let is_3p = model.to_lowercase().contains("claude")
        || model.to_lowercase().contains("gpt")
        || model.to_lowercase().contains("oss");

    let prefix = if is_3p { "3p" } else { "gemini" };
    let key_5h = format!("{}-5h", prefix);
    let key_w = format!("{}-weekly", prefix);

    let frac_5h = extract_quota_frac(&input, &key_5h).unwrap_or(1.0);
    let frac_w = extract_quota_frac(&input, &key_w).unwrap_or(1.0);

    let reset_5h = extract_quota_reset_secs(&input, &key_5h).unwrap_or(0);
    let reset_w = extract_quota_reset_secs(&input, &key_w).unwrap_or(0);

    let pct_5h = (frac_5h * 100.0).round() as u32;
    let pct_w = (frac_w * 100.0).round() as u32;

    let context_color = get_context_color(context_pct);
    let context_bar = build_progress_bar(context_pct, 5);
    let context_bar_colored = format!("{}{}{}", context_color, context_bar, WHITE);

    let bar_5h = build_progress_bar(pct_5h, 5);
    let bar_w = build_progress_bar(pct_w, 5);

    let r_5h_str = if reset_5h > 0 {
        format!(" (↻ {})", format_time_left(reset_5h))
    } else {
        "".to_string()
    };
    let r_w_str = if reset_w > 0 {
        format!(" (↻ {})", format_time_left(reset_w))
    } else {
        "".to_string()
    };

    let color_5h = get_quota_color(pct_5h);
    let color_w = get_quota_color(pct_w);

    let mut flags = [false, false, false, false, false]; // dir, branch, model, quotas, context
    let min_padding = 3;

    let get_parts = |flags: &[bool; 5]| {
        let cwd_display = if flags[0] {
            cwd_basename.to_string()
        } else if cwd.starts_with(&home) {
            cwd.replacen(&home, "~", 1)
        } else {
            cwd.clone()
        };

        let branch_display = if flags[1] {
            let mut b = branch_raw.clone();
            if b.len() > 15 {
                let parts: Vec<&str> = b.split('-').collect();
                if parts.len() >= 2 {
                    b = format!("{}-{}", parts[0], parts[1]);
                }
            }
            b
        } else {
            branch_raw.clone()
        };

        let model_display = if flags[2] {
            let mut m = model.clone();
            if let Some(idx) = m.rfind(" (") {
                m.truncate(idx);
            }
            m
        } else {
            model.clone()
        };

        let mut location = String::new();
        if !cwd_display.is_empty() {
            location.push_str(&format!(" {YELLOW}{}", cwd_display));
        }
        if branch_display != "none" && !branch_display.is_empty() {
            if !location.is_empty() {
                location.push(' ');
            }
            location.push_str(&format!("{WHITE}({}{}{WHITE})", MAGENTA, branch_display));
        }
        let location_display = if location.is_empty() {
            "".to_string()
        } else {
            format!("{} {RESET}│", location)
        };

        let quotas_display = if flags[3] {
            format!(
                "5h {}{}{WHITE} {}% - W {}{}{WHITE} {}%",
                color_5h, bar_5h, pct_5h, color_w, bar_w, pct_w
            )
        } else {
            format!(
                "5h {}{}{WHITE} {}%{} - W {}{}{WHITE} {}%{}",
                color_5h, bar_5h, pct_5h, r_5h_str, color_w, bar_w, pct_w, r_w_str
            )
        };

        let context_label = if flags[4] { "Ctx" } else { "Context" };

        let left_side = format!(
            "{}{WHITE} {} {} {}% {RESET}│ {WHITE} Quotas {} {RESET}",
            location_display, context_label, context_bar_colored, context_pct, quotas_display
        );

        let right_side = format!("{BLUE}{} - {} {RESET}", active_agents, model_display);

        (left_side, right_side)
    };

    let (mut left_side, mut right_side) = get_parts(&flags);

    if term_width > 0 {
        let mut i = 0;
        while i < 5 && visible_len(&left_side) + visible_len(&right_side) + min_padding > term_width
        {
            flags[i] = true;
            let (l, r) = get_parts(&flags);
            left_side = l;
            right_side = r;
            i += 1;
        }
    }

    let left_len = visible_len(&left_side);
    let right_len = visible_len(&right_side);

    let padding = if term_width > left_len + right_len {
        " ".repeat(term_width - left_len - right_len)
    } else {
        "   ".to_string()
    };

    let status_line = format!("{}{}{}", left_side, padding, right_side);

    print!("{}", status_line);
    let _ = io::stdout().flush();
}
