use serde::{Deserialize, Serialize};

// Format verified against the real server source (game-src/source/game/src/
// regen.cpp::read_line), not guessed - and against a real file
// (share/data/dungeon/dt_short/deviltower3_regen.txt: "ga 210 210 10 10 0 0
// 0s 100 1 1024"). One whitespace-separated entry per line:
//
//   <type> <center_x> <center_y> <radius_x> <radius_y> <z_section> <direction> <regen_time> <regen_percent> <max_count> <vnum>
//
// `type`'s first character selects the spawn kind ('m'=single mob, 'g'=group
// - second char 'a' means aggressive, 'r'=group-of-groups, 's'=anywhere on
// the map). An `e`xception entry is special: the server's own parser stops
// reading right after z_section for it, so it only has 5 fields
// (<e> <center_x> <center_y> <radius_x> <radius_y> <z_section>) - no
// direction/time/percent/count/vnum. `regen_percent` is parsed by the real
// game but never actually used anywhere in the C++ (dead field kept for
// file-format compatibility) - preserved here for a faithful round-trip,
// not because it does anything.
//
// The real engine tokenizes the whole file as one whitespace-delimited
// stream (newlines are just another separator to it), so in principle an
// entry could theoretically span across a line break - in practice every
// real file (like the sample above) puts exactly one entry per line, and
// this parser requires that same layout: parse-or-refuse rather than
// silently guessing at anything unusual.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegenSpawn {
    pub spawn_type: String,
    pub center_x: i32,
    pub center_y: i32,
    pub radius_x: i32,
    pub radius_y: i32,
    pub z_section: i32,
    pub direction: i32,
    pub regen_time: String,
    pub regen_percent: i32,
    pub max_count: i32,
    pub vnum: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegenException {
    pub center_x: i32,
    pub center_y: i32,
    pub radius_x: i32,
    pub radius_y: i32,
    pub z_section: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum RegenLine {
    Spawn(RegenSpawn),
    Exception(RegenException),
    /// Blank line or a `//`-comment line, preserved verbatim so editing one
    /// entry in a file doesn't strip the layout notes around it.
    Raw { text: String },
}

fn parse_i32(token: &str, line_no: usize, field: &str) -> Result<i32, String> {
    token
        .parse::<i32>()
        .map_err(|_| format!("Zeile {}: ungültiger Wert für {field}: '{token}'", line_no + 1))
}

pub fn parse(content: &str) -> Result<Vec<RegenLine>, String> {
    let mut out = Vec::new();
    for (line_no, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            out.push(RegenLine::Raw { text: line.to_string() });
            continue;
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        let spawn_type = tokens[0];
        let first_char = spawn_type.chars().next().unwrap_or('?');

        if first_char == 'e' {
            if tokens.len() != 6 {
                return Err(format!(
                    "Zeile {}: 'e'-Eintrag (Ausnahmezone) braucht 5 Werte nach dem Typ (x y radius_x radius_y z_section), gefunden {}",
                    line_no + 1,
                    tokens.len() - 1
                ));
            }
            out.push(RegenLine::Exception(RegenException {
                center_x: parse_i32(tokens[1], line_no, "center_x")?,
                center_y: parse_i32(tokens[2], line_no, "center_y")?,
                radius_x: parse_i32(tokens[3], line_no, "radius_x")?,
                radius_y: parse_i32(tokens[4], line_no, "radius_y")?,
                z_section: parse_i32(tokens[5], line_no, "z_section")?,
            }));
        } else if matches!(first_char, 'm' | 'g' | 'r' | 's') {
            if tokens.len() != 11 {
                return Err(format!(
                    "Zeile {}: Spawn-Eintrag braucht 10 Werte nach dem Typ (x y radius_x radius_y z_section direction zeit prozent max_anzahl vnum), gefunden {}",
                    line_no + 1,
                    tokens.len() - 1
                ));
            }
            out.push(RegenLine::Spawn(RegenSpawn {
                spawn_type: spawn_type.to_string(),
                center_x: parse_i32(tokens[1], line_no, "center_x")?,
                center_y: parse_i32(tokens[2], line_no, "center_y")?,
                radius_x: parse_i32(tokens[3], line_no, "radius_x")?,
                radius_y: parse_i32(tokens[4], line_no, "radius_y")?,
                z_section: parse_i32(tokens[5], line_no, "z_section")?,
                direction: parse_i32(tokens[6], line_no, "direction")?,
                regen_time: tokens[7].to_string(),
                regen_percent: parse_i32(tokens[8], line_no, "regen_percent")?,
                max_count: parse_i32(tokens[9], line_no, "max_count")?,
                vnum: parse_i32(tokens[10], line_no, "vnum")?,
            }));
        } else {
            return Err(format!(
                "Zeile {}: unbekannter Regen-Typ '{spawn_type}' (erwartet m/g/ga/r/s/e)",
                line_no + 1
            ));
        }
    }
    Ok(out)
}

pub fn serialize(lines: &[RegenLine]) -> String {
    let mut out = String::new();
    for line in lines {
        match line {
            RegenLine::Raw { text } => {
                out.push_str(text);
                out.push('\n');
            }
            RegenLine::Exception(e) => {
                out.push_str(&format!(
                    "e\t{}\t{}\t{}\t{}\t{}\n",
                    e.center_x, e.center_y, e.radius_x, e.radius_y, e.z_section
                ));
            }
            RegenLine::Spawn(s) => {
                out.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    s.spawn_type,
                    s.center_x,
                    s.center_y,
                    s.radius_x,
                    s.radius_y,
                    s.z_section,
                    s.direction,
                    s.regen_time,
                    s.regen_percent,
                    s.max_count,
                    s.vnum
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "// Etage 3\nga\t210\t210\t10\t10\t0\t0\t0s\t100\t1\t1024\n\ne\t100\t100\t5\t5\t0\n";

    #[test]
    fn parses_real_sample_line() {
        let lines = parse(SAMPLE).expect("should parse the real server sample");
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], RegenLine::Raw { text: "// Etage 3".to_string() });
        assert_eq!(
            lines[1],
            RegenLine::Spawn(RegenSpawn {
                spawn_type: "ga".to_string(),
                center_x: 210,
                center_y: 210,
                radius_x: 10,
                radius_y: 10,
                z_section: 0,
                direction: 0,
                regen_time: "0s".to_string(),
                regen_percent: 100,
                max_count: 1,
                vnum: 1024,
            })
        );
        assert_eq!(lines[2], RegenLine::Raw { text: String::new() });
        assert_eq!(
            lines[3],
            RegenLine::Exception(RegenException {
                center_x: 100,
                center_y: 100,
                radius_x: 5,
                radius_y: 5,
                z_section: 0,
            })
        );
    }

    #[test]
    fn round_trips_exactly() {
        let lines = parse(SAMPLE).unwrap();
        let rendered = serialize(&lines);
        let reparsed = parse(&rendered).unwrap();
        assert_eq!(lines, reparsed);
    }

    #[test]
    fn rejects_wrong_field_count_instead_of_guessing() {
        assert!(parse("m\t1\t2\t3\n").is_err());
        assert!(parse("e\t1\t2\t3\t4\t5\t6\n").is_err());
    }

    #[test]
    fn rejects_unknown_type() {
        assert!(parse("x\t1\t2\t3\t4\t5\t6\t7s\t8\t9\t10\n").is_err());
    }
}
