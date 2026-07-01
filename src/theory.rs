//! Music-theory primitives: notes, chords, scales, fretboard geometry and a
//! small demo song used to drive the UI.

use serde::{Deserialize, Serialize};

/// Names for the 12 pitch classes, using sharps.
pub const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// A note's place within a scale, named by its chromatic scale degree relative
/// to the root (root = `1`). E.g. a minor-pentatonic run reads `1 b3 4 5 b7`,
/// a major scale reads `1 2 3 4 5 6 7`. Arabic numerals are the convention for
/// individual scale tones (Roman numerals are reserved for chords).
pub const DEGREE_NAMES: [&str; 12] = [
    "1", "b2", "2", "b3", "3", "4", "b5", "5", "b6", "6", "b7", "7",
];

/// Scale-degree label for pitch class `pc` measured from `root`.
pub fn scale_degree_name(pc: u8, root: u8) -> &'static str {
    let offset = (pc + 12 - root % 12) % 12;
    DEGREE_NAMES[offset as usize]
}

/// How the fretboard labels each note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelMode {
    /// Absolute pitch name, e.g. "C", "D#".
    NoteName,
    /// Scale degree relative to the current scale root, e.g. "1", "b3".
    Degree,
}

/// Roman numerals for the seven scale degrees.
const ROMAN_NUMERALS: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];

/// Standard-tuning open-string pitch classes, ordered as they are drawn on the
/// board (top row = high e, bottom row = low E).
pub const STRINGS: [GuitarString; 6] = [
    GuitarString { label: "e", open_pc: 4 },  // high E
    GuitarString { label: "B", open_pc: 11 },
    GuitarString { label: "G", open_pc: 7 },
    GuitarString { label: "D", open_pc: 2 },
    GuitarString { label: "A", open_pc: 9 },
    GuitarString { label: "E", open_pc: 4 },  // low E
];

/// First and last fret shown on the board. Window is centered on fret 12.
pub const FRET_MIN: u8 = 7;
pub const FRET_MAX: u8 = 17;

#[derive(Clone, Copy, Debug)]
pub struct GuitarString {
    pub label: &'static str,
    /// Pitch class (0-11) of the open string.
    pub open_pc: u8,
}

/// Quality of a chord, expressed as semitone offsets from the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChordQuality {
    Major,
    Minor,
    Dominant7,
    Major7,
    Minor7,
}

impl ChordQuality {
    /// Semitone intervals from the root that make up the chord.
    pub fn intervals(self) -> &'static [u8] {
        match self {
            ChordQuality::Major => &[0, 4, 7],
            ChordQuality::Minor => &[0, 3, 7],
            ChordQuality::Dominant7 => &[0, 4, 7, 10],
            ChordQuality::Major7 => &[0, 4, 7, 11],
            ChordQuality::Minor7 => &[0, 3, 7, 10],
        }
    }

    /// Suffix appended to the root note when naming the chord.
    pub fn suffix(self) -> &'static str {
        match self {
            ChordQuality::Major => "",
            ChordQuality::Minor => "m",
            ChordQuality::Dominant7 => "7",
            ChordQuality::Major7 => "maj7",
            ChordQuality::Minor7 => "m7",
        }
    }
}

/// A scale type, expressed as semitone offsets from its root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScaleType {
    Major,
    NaturalMinor,
    MinorPentatonic,
    MajorPentatonic,
    Mixolydian,
}

impl ScaleType {
    pub fn intervals(self) -> &'static [u8] {
        match self {
            ScaleType::Major => &[0, 2, 4, 5, 7, 9, 11],
            ScaleType::NaturalMinor => &[0, 2, 3, 5, 7, 8, 10],
            ScaleType::MinorPentatonic => &[0, 3, 5, 7, 10],
            ScaleType::MajorPentatonic => &[0, 2, 4, 7, 9],
            ScaleType::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ScaleType::Major => "Major",
            ScaleType::NaturalMinor => "Natural Minor",
            ScaleType::MinorPentatonic => "Minor Pentatonic",
            ScaleType::MajorPentatonic => "Major Pentatonic",
            ScaleType::Mixolydian => "Mixolydian",
        }
    }
}

/// One section of the song: a chord plus the scale to solo over it, held for a
/// number of beats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    pub chord_root: u8,
    pub chord_quality: ChordQuality,
    pub scale_root: u8,
    pub scale_type: ScaleType,
    pub beats: usize,
}

impl Section {
    /// Display name of the chord, e.g. "Am" or "Gmaj7".
    pub fn chord_name(&self) -> String {
        format!(
            "{}{}",
            NOTE_NAMES[self.chord_root as usize % 12],
            self.chord_quality.suffix()
        )
    }

    /// The chord's place in a key, as a Roman numeral relative to `key_root`
    /// (e.g. in C major: Am -> "vi", F -> "IV", C -> "I", G7 -> "V7"). Case
    /// reflects quality (upper = major-ish, lower = minor-ish); a leading "b"
    /// marks a chromatic (non-diatonic) root.
    pub fn roman_numeral(&self, key_root: u8) -> String {
        let semis = (self.chord_root + 12 - key_root % 12) % 12;
        // Map the semitone distance to a major-scale degree plus accidental.
        let (degree_idx, accidental) = match semis {
            0 => (0, ""),
            1 => (1, "b"),
            2 => (1, ""),
            3 => (2, "b"),
            4 => (2, ""),
            5 => (3, ""),
            6 => (4, "b"),
            7 => (4, ""),
            8 => (5, "b"),
            9 => (5, ""),
            10 => (6, "b"),
            _ => (6, ""), // 11
        };

        let minorish = matches!(
            self.chord_quality,
            ChordQuality::Minor | ChordQuality::Minor7
        );
        let numeral = if minorish {
            ROMAN_NUMERALS[degree_idx].to_lowercase()
        } else {
            ROMAN_NUMERALS[degree_idx].to_string()
        };

        // The minor "m" is implied by lowercase, so only carry 7th info.
        let suffix = match self.chord_quality {
            ChordQuality::Major | ChordQuality::Minor => "",
            ChordQuality::Dominant7 | ChordQuality::Minor7 => "7",
            ChordQuality::Major7 => "maj7",
        };

        format!("{}{}{}", accidental, numeral, suffix)
    }

    /// Display name of the scale, e.g. "A Natural Minor".
    pub fn scale_name(&self) -> String {
        format!(
            "{} {}",
            NOTE_NAMES[self.scale_root as usize % 12],
            self.scale_type.label()
        )
    }

    /// Set of pitch classes (0-11) that belong to the chord.
    pub fn chord_pcs(&self) -> Vec<u8> {
        self.chord_quality
            .intervals()
            .iter()
            .map(|i| (self.chord_root + i) % 12)
            .collect()
    }

    /// Set of pitch classes (0-11) that belong to the scale.
    pub fn scale_pcs(&self) -> Vec<u8> {
        self.scale_type
            .intervals()
            .iter()
            .map(|i| (self.scale_root + i) % 12)
            .collect()
    }
}

/// Pitch class for a given string at a given fret.
pub fn pitch_class(open_pc: u8, fret: u8) -> u8 {
    (open_pc + fret) % 12
}

/// Parse a note name (with optional accidentals) into a pitch class (0-11).
/// Accepts sharps (`#`, `s`) and flats (`b`). Returns `None` if unrecognized.
fn parse_note_pc(s: &str) -> Option<u8> {
    let mut chars = s.chars();
    let letter = chars.next()?;
    let base = match letter.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let mut pc = base as i32;
    for c in chars {
        match c {
            '#' | 's' => pc += 1,
            'b' | 'B' => pc -= 1,
            _ => return None,
        }
    }
    Some(((pc % 12) + 12) as u8 % 12)
}

/// Map a chord-quality token (the part after the root) to a [`ChordQuality`].
/// Handles both Harte notation (`maj`, `min`, `min7`, `7`, `maj7`) and common
/// suffixes (`m`, `m7`, ``). Unknown/extended qualities fall back to the closest
/// triad (major or minor) so any input song still renders.
fn parse_quality(token: &str) -> ChordQuality {
    let t = token.trim().to_ascii_lowercase();
    // Strip a leading ':' separator used by Harte notation (e.g. "C:min7").
    let t = t.strip_prefix(':').unwrap_or(&t);
    match t {
        "" | "maj" | "major" | "M" => ChordQuality::Major,
        "min" | "minor" | "m" => ChordQuality::Minor,
        "7" | "dom7" | "dom" => ChordQuality::Dominant7,
        "maj7" | "major7" | "m7+5" => ChordQuality::Major7,
        "min7" | "m7" | "minor7" => ChordQuality::Minor7,
        _ => {
            // Best-effort: anything that looks minor/diminished → minor triad,
            // anything with a dominant 7 flavor → dominant 7, else major.
            if t.starts_with("min") || t.starts_with("m") || t.starts_with("dim") {
                if t.contains('7') {
                    ChordQuality::Minor7
                } else {
                    ChordQuality::Minor
                }
            } else if t.contains("maj7") {
                ChordQuality::Major7
            } else if t.contains('7') {
                ChordQuality::Dominant7
            } else {
                ChordQuality::Major
            }
        }
    }
}

/// Parse a chord label into `(root pitch class, quality)`. Supports plain
/// (`"Am"`, `"Gmaj7"`, `"C#m7"`), Harte (`"A:min"`, `"E:min7"`, `"G:7"`), and
/// slash chords (`"C/E"` → uses the chord root, ignores the bass). Returns
/// `None` for "no chord" markers (`"N"`, `"X"`, empty).
pub fn parse_chord_label(label: &str) -> Option<(u8, ChordQuality)> {
    let label = label.trim();
    if label.is_empty() || label == "N" || label == "X" || label.eq_ignore_ascii_case("nc") {
        return None;
    }
    // Drop any slash-bass annotation ("C/E", "A:min/b3").
    let head = label.split('/').next().unwrap_or(label);

    // Split root from quality. The root is the leading note name: a letter plus
    // any immediately-following accidentals.
    let bytes = head.as_bytes();
    let mut i = 1usize.min(head.len());
    while i < bytes.len() {
        match bytes[i] as char {
            '#' | 'b' | 's' => i += 1,
            _ => break,
        }
    }
    // Harte notation separates with ':'; handle that boundary too.
    let (root_str, qual_str) = head.split_at(i);
    let root = parse_note_pc(root_str)?;
    let quality = parse_quality(qual_str);
    Some((root, quality))
}

/// A chord placed on the audio timeline, in seconds. Produced by ingesting a
/// real song; consumed by the player to drive the highlighted `current` chord.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimedChord {
    pub chord_root: u8,
    pub chord_quality: ChordQuality,
    /// Start time in seconds (inclusive).
    pub start: f64,
    /// End time in seconds (exclusive).
    pub end: f64,
}

impl TimedChord {
    /// Convert to a [`Section`], picking a sensible solo scale for the chord and
    /// quantizing its duration to whole "beats" (seconds) for the timeline.
    pub fn to_section(&self) -> Section {
        let scale_type = match self.chord_quality {
            ChordQuality::Minor | ChordQuality::Minor7 => ScaleType::MinorPentatonic,
            ChordQuality::Dominant7 => ScaleType::Mixolydian,
            _ => ScaleType::MajorPentatonic,
        };
        let beats = (self.end - self.start).round().max(1.0) as usize;
        Section {
            chord_root: self.chord_root,
            chord_quality: self.chord_quality,
            scale_root: self.chord_root,
            scale_type,
            beats,
        }
    }
}

/// Number of scale "position" boxes the player can cycle through.
pub const POSITION_COUNT: usize = 5;

/// Width (in frets) of one scale-position box.
const POSITION_SPAN: u8 = 4;

/// Fret window `(start, end)` of the `index`-th scale-position box for a scale
/// rooted at `scale_root`.
///
/// Boxes are anchored to the scale tones that fall on the low‑E string within
/// the visible neck: position 0 starts at the lowest such anchor, each
/// subsequent position at the next anchor up. The span is `POSITION_SPAN` frets
/// wide, clamped to the visible window `[FRET_MIN, FRET_MAX]`. This approximates
/// the standard CAGED boxes inside the fixed fret window.
pub fn scale_position_span(scale_root: u8, scale_type: ScaleType, index: usize) -> (u8, u8) {
    let scale_pcs: Vec<u8> = scale_type
        .intervals()
        .iter()
        .map(|i| (scale_root + i) % 12)
        .collect();

    // Frets on the low‑E string (open pc 4) that land on a scale tone.
    let anchors: Vec<u8> = (FRET_MIN..=FRET_MAX)
        .filter(|&f| scale_pcs.contains(&pitch_class(4, f)))
        .collect();

    if anchors.is_empty() {
        return (FRET_MIN, FRET_MAX);
    }

    let i = index.min(anchors.len() - 1);
    let start = anchors[i];
    let end = (start + POSITION_SPAN).min(FRET_MAX);
    (start, end)
}

/// Number of inlay dots drawn on a fret marker (0 = none, 1 = single, 2 = double).
pub fn fret_marker(fret: u8) -> u8 {
    match fret {
        12 | 24 => 2,
        3 | 5 | 7 | 9 | 15 | 17 | 19 | 21 => 1,
        _ => 0,
    }
}

/// The five movable major chord shapes of the CAGED system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CagedShape {
    C,
    A,
    G,
    E,
    D,
}

impl CagedShape {
    /// All five shapes, in CAGED order.
    pub const ALL: [CagedShape; 5] = [
        CagedShape::C,
        CagedShape::A,
        CagedShape::G,
        CagedShape::E,
        CagedShape::D,
    ];

    /// Single-letter label, e.g. "C".
    pub fn letter(self) -> &'static str {
        match self {
            CagedShape::C => "C",
            CagedShape::A => "A",
            CagedShape::G => "G",
            CagedShape::E => "E",
            CagedShape::D => "D",
        }
    }

    /// Lower-case slug used for CSS classes, e.g. "c".
    pub fn slug(self) -> &'static str {
        match self {
            CagedShape::C => "c",
            CagedShape::A => "a",
            CagedShape::G => "g",
            CagedShape::E => "e",
            CagedShape::D => "d",
        }
    }

    /// Pitch class of the shape's nominal open-chord root (C, A, G, E, D).
    fn base_root(self) -> u8 {
        match self {
            CagedShape::C => 0,
            CagedShape::A => 9,
            CagedShape::G => 7,
            CagedShape::E => 4,
            CagedShape::D => 2,
        }
    }

    /// Open-chord fret per string, in `STRINGS` order (high-e first); `None`
    /// marks a muted string. Moving the whole pattern up by `shift` semitones
    /// transposes the shape to any root.
    fn open_frets(self) -> [Option<u8>; 6] {
        // Order matches STRINGS: [e, B, G, D, A, E].
        match self {
            CagedShape::C => [Some(0), Some(1), Some(0), Some(2), Some(3), None],
            CagedShape::A => [Some(0), Some(2), Some(2), Some(2), Some(0), None],
            CagedShape::G => [Some(3), Some(0), Some(0), Some(0), Some(2), Some(3)],
            CagedShape::E => [Some(0), Some(0), Some(1), Some(2), Some(2), Some(0)],
            CagedShape::D => [Some(2), Some(3), Some(2), Some(0), None, None],
        }
    }
}

/// One fretted note belonging to a CAGED shape placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CagedCell {
    /// Index into `STRINGS` (high-e first).
    pub string_index: usize,
    pub fret: u8,
    /// Whether this note is a root of the chord.
    pub is_root: bool,
}

/// One occurrence of a CAGED shape inside the visible fret window. A shape can
/// appear more than once (octave copies); each occurrence is its own placement
/// with its own bounding box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CagedPlacement {
    pub shape: CagedShape,
    pub cells: Vec<CagedCell>,
    /// Inclusive fret span of the box, already clamped to the visible window.
    pub min_fret: u8,
    pub max_fret: u8,
}

/// Compute the visible CAGED shape placements for a major chord rooted at
/// `root`. Shapes are clipped to `FRET_MIN..=FRET_MAX`, so this works for any
/// fret window — change only `FRET_MIN`/`FRET_MAX` to retune the board.
pub fn caged_shapes(root: u8) -> Vec<CagedPlacement> {
    let r = root % 12;
    let mut out = Vec::new();
    // Enough octave copies to cover any window up to FRET_MAX.
    let k_max = FRET_MAX as i32 / 12 + 1;

    for shape in CagedShape::ALL {
        let shift = (12 + r as i32 - shape.base_root() as i32) % 12; // 0..=11
        let frets = shape.open_frets();

        for k in -2..=k_max {
            let mut cells = Vec::new();
            for (i, open) in frets.iter().enumerate() {
                let Some(f) = open else { continue };
                let fret = *f as i32 + shift + 12 * k;
                if fret < FRET_MIN as i32 || fret > FRET_MAX as i32 {
                    continue;
                }
                let fret = fret as u8;
                let is_root = pitch_class(STRINGS[i].open_pc, fret) == r;
                cells.push(CagedCell {
                    string_index: i,
                    fret,
                    is_root,
                });
            }
            if cells.is_empty() {
                continue;
            }
            let min_fret = cells.iter().map(|c| c.fret).min().unwrap();
            let max_fret = cells.iter().map(|c| c.fret).max().unwrap();
            out.push(CagedPlacement {
                shape,
                cells,
                min_fret,
                max_fret,
            });
        }
    }
    out
}

/// Guess the song key from a list of timed chords. Scores all 24 major and
/// natural-minor keys by how much of the (duration-weighted) chord content is
/// diatonic to each, then breaks the inevitable relative-major/minor tie by
/// favoring the mode whose tonic chord is more prominent. Falls back to C major
/// for empty input.
pub fn guess_key(chords: &[TimedChord]) -> (u8, ScaleType) {
    if chords.is_empty() {
        return (0, ScaleType::Major);
    }

    // Duration weight of chords rooted on each pitch class (for tonic tie-break).
    let mut root_weight = [0.0f64; 12];
    for c in chords {
        root_weight[(c.chord_root % 12) as usize] += (c.end - c.start).max(0.0);
    }

    let candidates = [ScaleType::Major, ScaleType::NaturalMinor];
    let mut best: Option<(f64, f64, u8, ScaleType)> = None; // (diatonic, tonic_w, root, scale)

    for tonic in 0u8..12 {
        for scale in candidates {
            let pcs: Vec<u8> = scale.intervals().iter().map(|i| (tonic + i) % 12).collect();
            // Diatonic score: full weight when every chord tone fits the scale,
            // half when only the root fits.
            let mut diatonic = 0.0;
            for c in chords {
                let dur = (c.end - c.start).max(0.0);
                let tones: Vec<u8> = c
                    .chord_quality
                    .intervals()
                    .iter()
                    .map(|i| (c.chord_root + i) % 12)
                    .collect();
                let all_in = tones.iter().all(|t| pcs.contains(t));
                let root_in = pcs.contains(&(c.chord_root % 12));
                if all_in {
                    diatonic += dur;
                } else if root_in {
                    diatonic += dur * 0.5;
                }
            }
            let tonic_w = root_weight[tonic as usize];
            let better = match best {
                None => true,
                Some((bd, btw, _, _)) => {
                    diatonic > bd + 1e-6 || ((diatonic - bd).abs() <= 1e-6 && tonic_w > btw)
                }
            };
            if better {
                best = Some((diatonic, tonic_w, tonic, scale));
            }
        }
    }

    let (_, _, root, scale) = best.unwrap();
    (root, scale)
}

/// A whole song: a base key plus an ordered list of sections that loops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Song {
    /// Tonic pitch class of the song's key.
    pub key_root: u8,
    /// Scale type of the song's key (its diatonic set).
    pub key_scale: ScaleType,
    pub sections: Vec<Section>,
}

impl Song {
    /// Display name of the base key, e.g. "C Major".
    pub fn key_name(&self) -> String {
        format!(
            "{} {}",
            NOTE_NAMES[self.key_root as usize % 12],
            self.key_scale.label()
        )
    }

    /// Pitch classes that belong to the base key.
    pub fn key_pcs(&self) -> Vec<u8> {
        self.key_scale
            .intervals()
            .iter()
            .map(|i| (self.key_root + i) % 12)
            .collect()
    }
}

/// A simple looping demo progression in the key of C major / A minor.
pub fn demo_song() -> Song {
    Song {
        key_root: 0, // C
        key_scale: ScaleType::Major,
        sections: vec![
            Section {
                chord_root: 9, // A
                chord_quality: ChordQuality::Minor,
                scale_root: 9,
                scale_type: ScaleType::MinorPentatonic,
                beats: 4,
            },
            Section {
                chord_root: 5, // F
                chord_quality: ChordQuality::Major,
                scale_root: 5,
                scale_type: ScaleType::MajorPentatonic,
                beats: 4,
            },
            Section {
                chord_root: 0, // C
                chord_quality: ChordQuality::Major,
                scale_root: 0,
                scale_type: ScaleType::Major,
                beats: 4,
            },
            Section {
                chord_root: 4, // E (Em — diatonic iii chord of C major)
                chord_quality: ChordQuality::Minor,
                scale_root: 4,
                scale_type: ScaleType::MinorPentatonic,
                beats: 4,
            },
            Section {
                chord_root: 7, // G
                chord_quality: ChordQuality::Dominant7,
                scale_root: 7,
                scale_type: ScaleType::Mixolydian,
                beats: 4,
            },
        ],
    }
}

/// Total number of beats in one loop of the song.
pub fn song_length(song: &[Section]) -> usize {
    song.iter().map(|s| s.beats).sum()
}

/// Given the total beats elapsed, return the active section index and the
/// 1-based beat number within that section.
pub fn locate(song: &[Section], total_beats: usize) -> (usize, usize) {
    let len = song_length(song).max(1);
    let pos = total_beats % len;
    let mut acc = 0;
    for (i, s) in song.iter().enumerate() {
        if pos < acc + s.beats {
            return (i, pos - acc + 1);
        }
        acc += s.beats;
    }
    (0, 1)
}

/// A monotonically increasing index of how many sections have started, counting
/// across loops. Increments by one on every chord change. Used to drive the
/// timeline's continuous slide.
pub fn section_ordinal(song: &[Section], total_beats: usize) -> usize {
    let loop_beats = song_length(song).max(1);
    let loops = total_beats / loop_beats;
    let (idx, _) = locate(song, total_beats);
    loops * song.len().max(1) + idx
}
