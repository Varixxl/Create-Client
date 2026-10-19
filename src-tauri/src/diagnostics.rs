use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashDiagnosis {
    pub profile_id: String,
    pub timestamp: String,
    pub exit_code: Option<i32>,
    pub issue_category: String,
    pub summary: String,
    pub root_cause: String,
    pub suspected_mods: Vec<String>,
    pub suggested_fix: String,
    pub report_path: Option<String>,
    pub log_snippet: String,
}

/// Diagnoses the root cause of an unexpected game termination or crash.
/// Inspects recent crash reports, latest.log, launcher_log.txt, and mod conflict patterns.
pub fn diagnose_instance_crash(
    profile_id: &str,
    instance_dir: &Path,
    exit_code: Option<i32>,
) -> CrashDiagnosis {
    let now = chrono_now();

    // 1. First, check if there is a recent crash report in crash-reports/
    let crash_dir = instance_dir.join("crash-reports");
    let mut newest_report: Option<(SystemTime, PathBuf)> = None;
    if crash_dir.exists() {
        if let Ok(entries) = fs::read_dir(&crash_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("txt") {
                    if let Ok(modified) = p.metadata().and_then(|m| m.modified()) {
                        // Look for reports modified within the last 15 minutes
                        let is_recent = modified
                            .elapsed()
                            .map(|d| d.as_secs() < 900)
                            .unwrap_or(false);
                        if is_recent && newest_report.as_ref().map_or(true, |(t, _)| modified > *t) {
                            newest_report = Some((modified, p));
                        }
                    }
                }
            }
        }
    }

    if let Some((_, report_path)) = newest_report {
        if let Ok(content) = fs::read_to_string(&report_path) {
            if let Some(mut diag) = parse_crash_report(profile_id, &content, exit_code, &report_path) {
                diag.timestamp = now.clone();
                save_diagnosis(instance_dir, &diag);
                return diag;
            }
        }
    }

    // 2. Check logs/latest.log and launcher_log.txt
    let launcher_log = instance_dir.join("launcher_log.txt");
    let latest_log = instance_dir.join("logs").join("latest.log");

    let mut log_text = String::new();
    if launcher_log.exists() {
        if let Ok(text) = fs::read_to_string(&launcher_log) {
            log_text.push_str(&text);
            log_text.push('\n');
        }
    }
    if latest_log.exists() {
        if let Ok(text) = fs::read_to_string(&latest_log) {
            // Take the tail of latest.log to avoid allocating huge memory
            let lines: Vec<&str> = text.lines().rev().take(300).collect();
            let tail = lines.into_iter().rev().collect::<Vec<_>>().join("\n");
            log_text.push_str(&tail);
        }
    }

    if let Some(mut diag) = parse_log_errors(profile_id, &log_text, exit_code) {
        diag.timestamp = now.clone();
        save_diagnosis(instance_dir, &diag);
        return diag;
    }

    // 3. Fallback generic diagnosis
    let last_lines = extract_error_snippet(&log_text, 15);
    let diag = CrashDiagnosis {
        profile_id: profile_id.to_string(),
        timestamp: now,
        exit_code,
        issue_category: "Unexpected Termination".to_string(),
        summary: format!(
            "Minecraft process exited unexpectedly (Exit code: {}).",
            exit_code.map(|c| c.to_string()).unwrap_or_else(|| "Unknown".to_string())
        ),
        root_cause: if !last_lines.is_empty() {
            last_lines.clone()
        } else {
            "Process terminated without emitting standard crash report or fatal error log.".to_string()
        },
        suspected_mods: Vec::new(),
        suggested_fix: "Check your allocated RAM and verify your graphics drivers. Click 'Check Logs' to view the full game console.".to_string(),
        report_path: None,
        log_snippet: last_lines,
    };

    save_diagnosis(instance_dir, &diag);
    diag
}

fn parse_crash_report(
    profile_id: &str,
    content: &str,
    exit_code: Option<i32>,
    report_path: &Path,
) -> Option<CrashDiagnosis> {
    let mut description = String::new();
    let mut suspected_mods = Vec::new();
    let mut exception_message = String::new();
    let mut root_cause = String::new();

    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("Description:") {
            description = line.trim_start_matches("Description:").trim().to_string();
        } else if line.contains("Suspected Mods:") {
            for sub_line in lines.iter().skip(i + 1).take(8) {
                let trimmed = sub_line.trim();
                if trimmed.starts_with("at ") || trimmed.is_empty() || trimmed.starts_with("--") {
                    break;
                }
                suspected_mods.push(trimmed.to_string());
            }
        } else if line.starts_with("Exception message:") || line.starts_with("Failure message:") {
            exception_message = line.to_string();
        } else if line.starts_with("Caused by:") && root_cause.is_empty() {
            root_cause = line.to_string();
        }
    }

    // Check specific known crash signatures
    if content.contains("com.seibel.distanthorizons.core.config.eventHandlers.presets.ThreadPresetConfigEventHandler") {
        return Some(CrashDiagnosis {
            profile_id: profile_id.to_string(),
            timestamp: String::new(),
            exit_code,
            issue_category: "Mod Configuration Lifecycle Conflict".to_string(),
            summary: "Distant Horizons encountered an uninitialized configuration class during client setup.".to_string(),
            root_cause: "Premature class initialization of ThreadPresetConfigEventHandler before ConfigHandler finished setup.".to_string(),
            suspected_mods: vec!["Distant Horizons (distanthorizons)".to_string(), "KubeJS (kubejs)".to_string()],
            suggested_fix: "Graphics Presets script updated to defer polling until client and world initialization complete.".to_string(),
            report_path: Some(report_path.to_string_lossy().to_string()),
            log_snippet: extract_first_n_lines(content, 30),
        });
    }

    if content.contains("OutOfMemoryError") || content.contains("Java heap space") {
        return Some(CrashDiagnosis {
            profile_id: profile_id.to_string(),
            timestamp: String::new(),
            exit_code,
            issue_category: "Out of Memory".to_string(),
            summary: "The game ran out of allocated Java heap memory.".to_string(),
            root_cause: "java.lang.OutOfMemoryError: Java heap space".to_string(),
            suspected_mods,
            suggested_fix: "Increase RAM allocated to this profile in Client Settings (e.g. 8 GB - 12 GB).".to_string(),
            report_path: Some(report_path.to_string_lossy().to_string()),
            log_snippet: extract_first_n_lines(content, 30),
        });
    }

    if content.contains("DuplicateModsFoundException") {
        return Some(CrashDiagnosis {
            profile_id: profile_id.to_string(),
            timestamp: String::new(),
            exit_code,
            issue_category: "Duplicate Mods Detected".to_string(),
            summary: "Multiple versions of the same mod were detected in the mods directory.".to_string(),
            root_cause: if !exception_message.is_empty() { exception_message } else { "DuplicateModsFoundException".to_string() },
            suspected_mods,
            suggested_fix: "Remove older duplicate .jar files from the instance's mods folder.".to_string(),
            report_path: Some(report_path.to_string_lossy().to_string()),
            log_snippet: extract_first_n_lines(content, 30),
        });
    }

    let summary = if !description.is_empty() {
        format!("Mod loading error: {}", description)
    } else {
        "Minecraft crashed during initialization.".to_string()
    };

    let cause = if !root_cause.is_empty() {
        root_cause
    } else if !exception_message.is_empty() {
        exception_message
    } else {
        extract_first_n_lines(content, 10)
    };

    Some(CrashDiagnosis {
        profile_id: profile_id.to_string(),
        timestamp: String::new(),
        exit_code,
        issue_category: "Minecraft Crash Report".to_string(),
        summary,
        root_cause: cause,
        suspected_mods,
        suggested_fix: "Review the suspected mods and check for version incompatibilities.".to_string(),
        report_path: Some(report_path.to_string_lossy().to_string()),
        log_snippet: extract_first_n_lines(content, 35),
    })
}

fn parse_log_errors(
    profile_id: &str,
    log_text: &str,
    exit_code: Option<i32>,
) -> Option<CrashDiagnosis> {
    // 1. Fabric Java version mismatch
    if log_text.contains("Incompatible mods found!") || log_text.contains("FormattedException") {
        let mut missing_details = Vec::new();
        let mut req_java = 21;
        let mut cur_java = 21;

        for line in log_text.lines() {
            if line.contains("Replace 'OpenJDK 64-Bit Server VM' (java)") {
                // e.g. Replace 'OpenJDK 64-Bit Server VM' (java) 21 with version 25 or later.
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (idx, p) in parts.iter().enumerate() {
                    if *p == "version" {
                        if let Some(next) = parts.get(idx + 1) {
                            if let Ok(v) = next.parse::<u32>() {
                                req_java = v;
                            }
                        }
                    } else if p.chars().all(|c| c.is_numeric()) {
                        if let Ok(v) = p.parse::<u32>() {
                            if v >= 8 && v <= 30 {
                                cur_java = v;
                            }
                        }
                    }
                }
            }
            if line.trim().starts_with("- Mod '") {
                missing_details.push(line.trim().to_string());
            }
        }

        if !missing_details.is_empty() || req_java != cur_java {
            return Some(CrashDiagnosis {
                profile_id: profile_id.to_string(),
                timestamp: String::new(),
                exit_code,
                issue_category: "Java Version Incompatibility".to_string(),
                summary: format!(
                    "Installed mods require Java {} or later, but the instance was launched with Java {}.",
                    req_java, cur_java
                ),
                root_cause: missing_details.join("\n"),
                suspected_mods: missing_details
                    .iter()
                    .filter_map(|s| s.split('\'').nth(1).map(|m| m.to_string()))
                    .collect(),
                suggested_fix: format!(
                    "Automatic remediation enabled: Create Client has provisioned Java {}. Relaunching will automatically use the correct runtime.",
                    req_java
                ),
                report_path: None,
                log_snippet: extract_error_snippet(log_text, 25),
            });
        }
    }

    // 2. JVM option errors (e.g. Unrecognized VM option 'ZGenerational' on Java 17)
    if log_text.contains("Unrecognized VM option 'ZGenerational'") {
        return Some(CrashDiagnosis {
            profile_id: profile_id.to_string(),
            timestamp: String::new(),
            exit_code,
            issue_category: "JVM Argument Conflict".to_string(),
            summary: "JVM flag '-XX:+ZGenerational' is not supported on Java versions prior to 21.".to_string(),
            root_cause: "Unrecognized VM option 'ZGenerational' on Java 17 runtime.".to_string(),
            suspected_mods: Vec::new(),
            suggested_fix: "The launcher has updated JVM flags to automatically omit ZGenerational on Java 17 and use G1GC.".to_string(),
            report_path: None,
            log_snippet: extract_error_snippet(log_text, 15),
        });
    }

    // 3. GLFW / OpenGL crashes
    if log_text.contains("GLFW error 65542") || log_text.contains("WGL:") || log_text.contains("GLX:") {
        return Some(CrashDiagnosis {
            profile_id: profile_id.to_string(),
            timestamp: String::new(),
            exit_code,
            issue_category: "OpenGL / Display Driver Error".to_string(),
            summary: "The graphics system failed to initialize an OpenGL context.".to_string(),
            root_cause: "GLFW window or display driver failure.".to_string(),
            suspected_mods: Vec::new(),
            suggested_fix: "Verify that your graphics drivers are up to date and display compositor is working properly.".to_string(),
            report_path: None,
            log_snippet: extract_error_snippet(log_text, 15),
        });
    }

    None
}

fn extract_first_n_lines(text: &str, count: usize) -> String {
    text.lines().take(count).collect::<Vec<_>>().join("\n")
}

fn extract_error_snippet(log_text: &str, count: usize) -> String {
    let mut relevant: Vec<&str> = Vec::new();
    for line in log_text.lines().rev() {
        if line.contains("ERROR")
            || line.contains("FATAL")
            || line.contains("Exception")
            || line.contains("Caused by")
            || line.contains("Error")
            || line.contains("Incompatible")
        {
            relevant.push(line);
            if relevant.len() >= count {
                break;
            }
        }
    }
    if relevant.is_empty() {
        log_text.lines().rev().take(count).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n")
    } else {
        relevant.into_iter().rev().collect::<Vec<_>>().join("\n")
    }
}

fn chrono_now() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    format!("{}", duration.as_secs())
}

fn save_diagnosis(instance_dir: &Path, diag: &CrashDiagnosis) {
    let txt_path = instance_dir.join("crash_diagnosis.txt");
    let json_path = instance_dir.join("crash_diagnosis.json");

    let text_content = format!(
        "================================================================================\n\
         CREATE CLIENT CRASH DIAGNOSIS REPORT\n\
         ================================================================================\n\
         Profile ID:      {}\n\
         Timestamp:       {}\n\
         Exit Code:       {}\n\
         Category:        {}\n\
         --------------------------------------------------------------------------------\n\
         SUMMARY:\n\
         {}\n\n\
         ROOT CAUSE:\n\
         {}\n\n\
         SUSPECTED MODS:\n\
         {}\n\n\
         SUGGESTED ACTION / FIX:\n\
         {}\n\
         --------------------------------------------------------------------------------\n\
         LOG / REPORT SNIPPET:\n\
         {}\n\
         ================================================================================\n",
        diag.profile_id,
        diag.timestamp,
        diag.exit_code.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string()),
        diag.issue_category,
        diag.summary,
        diag.root_cause,
        if diag.suspected_mods.is_empty() { "None identified".to_string() } else { diag.suspected_mods.join(", ") },
        diag.suggested_fix,
        diag.log_snippet
    );

    let _ = fs::write(&txt_path, text_content);
    if let Ok(json_str) = serde_json::to_string_pretty(diag) {
        let _ = fs::write(&json_path, json_str);
    }
}
