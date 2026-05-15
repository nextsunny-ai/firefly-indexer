// Naming validation + bulk rename plan.
//
// Adobe Firefly dataset standard pattern:
//   {project}_{seq}_M{module}_{ver}_{NNNNN}.{ext}
//   e.g. 26Q2_CK_M3_3_00001.jpg
//
// Originals are NEVER modified — rename copies to a sidecar folder
// (rule #17 preserve old work).

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

static STRICT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^(?P<project>\d{2}Q\d)_(?P<seq>[A-Z]{2})_M(?P<module>\d+)_(?P<ver>\d+)_(?P<num>\d{4,6})\.(?P<ext>jpe?g|png)$").unwrap()
});

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScanReport {
    pub folder: String,
    pub total: usize,
    pub conforming: Vec<String>,
    pub nonconforming: Vec<String>,
    pub other_files: Vec<String>,
    pub deduced_project: Option<String>,
    pub deduced_seq: Option<String>,
    pub deduced_module: Option<u32>,
    pub deduced_ver: Option<u32>,
}

pub fn scan_folder(folder: &Path) -> ScanReport {
    let mut report = ScanReport {
        folder: folder.to_string_lossy().to_string(),
        total: 0,
        conforming: Vec::new(),
        nonconforming: Vec::new(),
        other_files: Vec::new(),
        deduced_project: None,
        deduced_seq: None,
        deduced_module: None,
        deduced_ver: None,
    };
    if !folder.is_dir() {
        return report;
    }
    let mut entries: Vec<PathBuf> = match fs::read_dir(folder) {
        Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.is_file()).collect(),
        Err(_) => return report,
    };
    entries.sort();

    for p in entries {
        let name = match p.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let ext = p.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
        if !IMAGE_EXTS.contains(&ext.as_str()) {
            report.other_files.push(name);
            continue;
        }
        report.total += 1;
        if let Some(caps) = STRICT_RE.captures(&name) {
            report.conforming.push(name.clone());
            if report.deduced_project.is_none() {
                report.deduced_project = Some(caps.name("project").unwrap().as_str().to_ascii_uppercase());
                report.deduced_seq = Some(caps.name("seq").unwrap().as_str().to_ascii_uppercase());
                report.deduced_module = caps.name("module").unwrap().as_str().parse().ok();
                report.deduced_ver = caps.name("ver").unwrap().as_str().parse().ok();
            }
        } else {
            report.nonconforming.push(name);
        }
    }
    report
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenameRequest {
    pub folder: String,
    pub output_folder: String,
    pub project: String,    // "26Q2"
    pub seq: String,        // "CK"
    pub module: u32,        // 3
    pub ver: u32,           // 3
    pub start_index: u32,   // default 1
    pub num_width: usize,   // default 5
    pub only_nonconforming: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenameRow {
    pub original_name: String,
    pub new_name: String,
    pub original_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RenameReport {
    pub copied: usize,
    pub mapping_csv: String,
    pub rows: Vec<RenameRow>,
}

pub fn apply_rename(req: &RenameRequest) -> Result<RenameReport, String> {
    let folder = PathBuf::from(&req.folder);
    let out = PathBuf::from(&req.output_folder);
    fs::create_dir_all(&out).map_err(|e| format!("create_dir_all: {e}"))?;

    let scan = scan_folder(&folder);
    let mut files: Vec<String> = if req.only_nonconforming {
        scan.nonconforming.clone()
    } else {
        let mut v = scan.nonconforming.clone();
        v.extend(scan.conforming.clone());
        v
    };
    files.sort();

    let prefix = format!("{}_{}_M{}_{}", req.project.to_uppercase(), req.seq.to_uppercase(), req.module, req.ver);
    let mut rows = Vec::new();
    let mut idx = req.start_index;
    for name in &files {
        let src = folder.join(name);
        let mut ext = src.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).unwrap_or_default();
        if ext == "jpeg" {
            ext = "jpg".to_string();
        }
        let new_name = format!("{}_{:0width$}.{}", prefix, idx, ext, width = req.num_width);
        let dst = out.join(&new_name);
        if dst.exists() {
            continue;
        }
        fs::copy(&src, &dst).map_err(|e| format!("copy {}: {}", src.display(), e))?;
        rows.push(RenameRow {
            original_name: name.clone(),
            new_name: new_name.clone(),
            original_path: src.to_string_lossy().to_string(),
        });
        idx += 1;
    }

    let csv_path = out.join("_rename_mapping.csv");
    let mut csv = String::from("original_filename,new_filename,original_path\n");
    for r in &rows {
        csv.push_str(&format!("\"{}\",\"{}\",\"{}\"\n",
            r.original_name.replace('"', "\"\""),
            r.new_name.replace('"', "\"\""),
            r.original_path.replace('"', "\"\""),
        ));
    }
    fs::write(&csv_path, csv).map_err(|e| format!("write csv: {e}"))?;

    Ok(RenameReport {
        copied: rows.len(),
        mapping_csv: csv_path.to_string_lossy().to_string(),
        rows,
    })
}
