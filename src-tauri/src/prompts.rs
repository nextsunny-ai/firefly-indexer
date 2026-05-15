// Vision prompts — kept SHORT (cmdline arg) + spec packed into user_msg (stdin).
//
// Adobe Firefly training-data schema (matches sample A000223986_3.xlsx):
//   10 columns: filename, story, sceneid, shotid, title,
//               characterId, characterId_2, caption, edit_instruction, edit_type
//   characterid sheet: character_id, ethnicity, age, gender

pub const SYSTEM_PROMPT: &str = "You are a strict JSON formatter. \
Read images using the Read tool when asked, then return ONLY a valid JSON object \
matching the schema described in the user message. \
No prose, no explanation, no markdown code fence.";

const SPEC_BODY: &str = r#"REQUIRED OUTPUT JSON SHAPE:
{
  "characters": [
    {"character_id": "<id>", "ethnicity": "<Asian|Caucasian|Black|Hispanic|...>",
     "age": "<infant|child|teen|young adult|adult|middle-aged|senior>",
     "gender": "<female|male>"}
  ],
  "shots": [
    {"shotid": <int>, "title": "<see rules>", "characterId": "<id or null>",
     "characterId_2": "<id or null>", "caption": "<English, 1 sentence, 15-30 words>",
     "edit_instruction": "<English imperative or null>", "edit_type": "<see rules>"}
  ]
}

# title rules (MATCH the sample format EXACTLY)
- shotid 1 = always exactly "full_scene"
- background-only shot = exactly "background_isolated"
- single human shot = "{role}_{n}_isolated"
    - role in {woman, man, boy, girl, baby}
    - n is 0-BASED within the scene (first woman = woman_0_isolated, second = woman_1_isolated)
    - INDEPENDENT of characterId number
- single object shot = "{object}_{n}_isolated"
    - object = short snake_case noun (wok_pan, soap_dispenser, bubble_gun, frying_pan, kitchen_knife)
    - n is 0-BASED, starting at 0 (single wok = "wok_pan_0_isolated")
- NEVER drop the "_n_" segment. "wok_pan_isolated" is WRONG.

# edit_type rules
- shotid 1 => "composition"
    edit_instruction = describe placement of every subject relative to the background,
    one imperative sentence (start with: "Place ...", "Position ...", "Set ...").
- isolated shot SAME background as full_scene => "object_removal"
    edit_instruction = "Remove X and Y; keep Z."
- isolated human shot in DIFFERENT background => "character_reference"
    edit_instruction = "Isolate the {subject} on a separate reference background ({describe})."
- isolated object shot in DIFFERENT background => "object_reference"
    edit_instruction = similar to character_reference

# characterId rules
- compare each person to the existing character DB (face, age range, gender, ethnicity)
- REUSE that id if matched
- else MINT a new id "{scene_prefix}_f{n}_{age_int}" for female or "_m{n}_{age_int}" for male
- n is 1-BASED for characterId (first female = f1, second = f2)
- age_int bucketing (use visible cues — face maturity, build, attire, accessories):
    1 = infant (0-2)
    5 = child (3-9, small build, child-like proportions, school-age toys)
    10 = teen (10-17, adolescent, past childhood but pre-adult)
    20 = young adult (18-25)
    30 = adult (26-40, mature, possibly glasses, settled clothing)
    50 = middle-aged (41-60)
    70 = senior (61+)
- include EVERY newly minted character in the "characters" array; do NOT include reused ones
- characterId_2 is only set on full_scene rows with TWO people

# ethnicity default
- This dataset is a Korean production for Adobe Korea.
- DEFAULT ethnicity = "Asian" unless clearly non-Asian (e.g. unmistakable European/African features).
- Hair color alone is NOT a basis for non-Asian classification (dyed hair is common).

# caption style (MATCH the sample tone)
- ENGLISH, exactly ONE sentence, 15-30 words, factual, present tense.
- Describe who/what + key clothing/color/material + immediate surroundings.
- No camera-talk ("a shot of"). No interpretation ("she looks tired").
- Start patterns:
    full_scene 2 people => "A {p1} and a {p2} {verb} {object} on a {setting}."
    background_isolated => "The {setting} are empty, with no people or props present."
    {role}_{n}_isolated => "A {age} {gender} with {feature} wears a {clothing} ..."
    {object}_{n}_isolated => "A {color/material} {object} {state} on a {surface}."

# Output
- Return ONLY the JSON object. No prose. No markdown fence. No explanation."#;

/// Build the per-scene user message. `image_paths` are absolute paths in the
/// order they should appear (shotid 1..N).
pub fn build_user_message(
    image_paths: &[String],
    scene_prefix: &str,
    existing_characters: &serde_json::Value,
) -> String {
    let db_json = serde_json::to_string(existing_characters).unwrap_or_else(|_| "[]".to_string());
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!("scene_prefix = {scene_prefix}"));
    lines.push(format!("image_count = {}", image_paths.len()));
    lines.push(String::new());
    lines.push(SPEC_BODY.to_string());
    lines.push(String::new());
    lines.push(format!("Existing character DB (JSON array): {db_json}"));
    lines.push(String::new());
    lines.push("Use the Read tool to view each image in the order listed, then produce the JSON:".to_string());
    for (i, p) in image_paths.iter().enumerate() {
        lines.push(format!("  shotid={}  ->  {}", i + 1, p));
    }
    lines.push(String::new());
    lines.push("Return ONLY the JSON object. No prose.".to_string());
    lines.join("\n")
}
