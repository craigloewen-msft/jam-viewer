//! Music-theory primitives: notes, chords, scales, fretboard geometry and a
//! small demo song used to drive the UI.

/// Names for the 12 pitch classes, using sharps.
pub const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Number of inlay dots drawn on a fret marker (0 = none, 1 = single, 2 = double).
pub fn fret_marker(fret: u8) -> u8 {
    match fret {
        12 | 24 => 2,
        3 | 5 | 7 | 9 | 15 | 17 | 19 | 21 => 1,
        _ => 0,
    }
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
