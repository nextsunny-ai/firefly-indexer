// Excel writer for the Adobe Firefly Indexer output schema.
//
// Two sheets:
//   1. "scenes"      — 10 columns (filename..edit_type)
//   2. "characterid" — 4 columns (character_id, ethnicity, age, gender)
//
// Append-per-scene so a crashed run leaves a valid partial xlsx.
// Newly-minted characters get a yellow fill (review marker).

use rust_xlsxwriter::{Color, Format, Workbook, Worksheet};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShotRow {
    pub filename: String,
    pub story: Option<String>,
    pub sceneid: u32,
    pub shotid: u32,
    pub title: String,
    pub character_id: Option<String>,
    pub character_id_2: Option<String>,
    pub caption: String,
    pub edit_instruction: Option<String>,
    pub edit_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CharRow {
    pub character_id: String,
    pub ethnicity: String,
    pub age: String,
    pub gender: String,
}

const SCENES_HEADERS: [&str; 10] = [
    "filename", "story", "sceneid", "shotid", "title",
    "characterId", "characterId_2", "caption", "edit_instruction", "edit_type",
];
const CHAR_HEADERS: [&str; 4] = ["character_id", "ethnicity", "age", "gender"];

pub struct ExcelWriter {
    path: String,
    wb: Workbook,
    scenes_row: u32,
    chars_row: u32,
    header_fmt: Format,
    body_fmt: Format,
    new_char_fmt: Format,
}

impl ExcelWriter {
    pub fn create_new(path: &Path) -> Result<Self, String> {
        let mut wb = Workbook::new();

        let header_fmt = Format::new()
            .set_bold()
            .set_font_color(Color::White)
            .set_background_color(Color::RGB(0x305496))
            .set_align(rust_xlsxwriter::FormatAlign::Center)
            .set_font_name("Arial");
        let body_fmt = Format::new()
            .set_text_wrap()
            .set_align(rust_xlsxwriter::FormatAlign::Top)
            .set_font_name("Arial");
        let new_char_fmt = Format::new()
            .set_background_color(Color::RGB(0xFFFF00))
            .set_font_name("Arial");

        // scenes sheet
        {
            let ws = wb.add_worksheet().set_name("scenes").map_err(|e| e.to_string())?;
            write_header(ws, &SCENES_HEADERS, &header_fmt)?;
            apply_scenes_widths(ws);
        }
        // characterid sheet
        {
            let ws = wb.add_worksheet().set_name("characterid").map_err(|e| e.to_string())?;
            write_header(ws, &CHAR_HEADERS, &header_fmt)?;
            ws.set_column_width(0, 26.0).map_err(|e| e.to_string())?;
            ws.set_column_width(1, 12.0).map_err(|e| e.to_string())?;
            ws.set_column_width(2, 14.0).map_err(|e| e.to_string())?;
            ws.set_column_width(3, 10.0).map_err(|e| e.to_string())?;
        }

        let me = ExcelWriter {
            path: path.to_string_lossy().to_string(),
            wb,
            scenes_row: 1,
            chars_row: 1,
            header_fmt,
            body_fmt,
            new_char_fmt,
        };
        me.save_quiet()?;
        Ok(me)
    }

    fn save_quiet(&self) -> Result<(), String> {
        // workbook::save consumes &mut self, so callers manage their own save cycle
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), String> {
        self.wb.save(&self.path).map_err(|e| e.to_string())
    }

    pub fn append_shots(&mut self, rows: &[ShotRow]) -> Result<(), String> {
        let ws = self.wb.worksheet_from_name("scenes").map_err(|e| e.to_string())?;
        for r in rows {
            let row = self.scenes_row;
            ws.write_string_with_format(row, 0, &r.filename, &self.body_fmt).map_err(|e| e.to_string())?;
            if let Some(s) = &r.story {
                ws.write_string_with_format(row, 1, s, &self.body_fmt).map_err(|e| e.to_string())?;
            }
            ws.write_number_with_format(row, 2, r.sceneid as f64, &self.body_fmt).map_err(|e| e.to_string())?;
            ws.write_number_with_format(row, 3, r.shotid as f64, &self.body_fmt).map_err(|e| e.to_string())?;
            ws.write_string_with_format(row, 4, &r.title, &self.body_fmt).map_err(|e| e.to_string())?;
            if let Some(s) = &r.character_id   { ws.write_string_with_format(row, 5, s, &self.body_fmt).map_err(|e| e.to_string())?; }
            if let Some(s) = &r.character_id_2 { ws.write_string_with_format(row, 6, s, &self.body_fmt).map_err(|e| e.to_string())?; }
            ws.write_string_with_format(row, 7, &r.caption, &self.body_fmt).map_err(|e| e.to_string())?;
            if let Some(s) = &r.edit_instruction { ws.write_string_with_format(row, 8, s, &self.body_fmt).map_err(|e| e.to_string())?; }
            if let Some(s) = &r.edit_type        { ws.write_string_with_format(row, 9, s, &self.body_fmt).map_err(|e| e.to_string())?; }
            ws.set_row_height(row, 80.0).map_err(|e| e.to_string())?;
            self.scenes_row += 1;
        }
        Ok(())
    }

    pub fn append_chars(&mut self, chars: &[CharRow], known: &mut std::collections::HashSet<String>) -> Result<usize, String> {
        let ws = self.wb.worksheet_from_name("characterid").map_err(|e| e.to_string())?;
        let mut new_count = 0usize;
        for c in chars {
            if known.contains(&c.character_id) { continue; }
            let row = self.chars_row;
            ws.write_string_with_format(row, 0, &c.character_id, &self.new_char_fmt).map_err(|e| e.to_string())?;
            ws.write_string_with_format(row, 1, &c.ethnicity,     &self.new_char_fmt).map_err(|e| e.to_string())?;
            ws.write_string_with_format(row, 2, &c.age,           &self.new_char_fmt).map_err(|e| e.to_string())?;
            ws.write_string_with_format(row, 3, &c.gender,        &self.new_char_fmt).map_err(|e| e.to_string())?;
            known.insert(c.character_id.clone());
            new_count += 1;
            self.chars_row += 1;
        }
        Ok(new_count)
    }
}

fn write_header(ws: &mut Worksheet, headers: &[&str], fmt: &Format) -> Result<(), String> {
    for (i, h) in headers.iter().enumerate() {
        ws.write_string_with_format(0, i as u16, *h, fmt).map_err(|e| e.to_string())?;
    }
    ws.set_row_height(0, 22.0).map_err(|e| e.to_string())?;
    Ok(())
}

fn apply_scenes_widths(ws: &mut Worksheet) {
    let widths: [(u16, f64); 10] = [
        (0, 26.0), (1, 8.0), (2, 9.0), (3, 8.0), (4, 26.0),
        (5, 24.0), (6, 24.0), (7, 55.0), (8, 60.0), (9, 18.0),
    ];
    for (col, w) in widths {
        let _ = ws.set_column_width(col, w);
    }
}
