use serde::{Deserialize, Serialize};

// cube.txt format - source/game/src/cube.cpp (Cube_load/FN_check_cube_data),
// loaded from `<LocaleService_GetBasePath()>/cube.txt`. Verified against the
// real server source this session (2026-08-14): same base directory as
// special_item_group.txt (both build their path off the identical
// `LocaleService_GetBasePath()` call in `item_manager_read_tables.cpp`/
// `cube.cpp`), so this project's existing default
// `/usr/home/game/share/special_item_group.txt` implies
// `/usr/home/game/share/cube.txt` as the sibling default.
//
// NOT yet byte-verified against a real live `cube.txt` (unlike
// mob_drop_item.txt/special_item_group.txt, which were both checked against
// the user's actual server file with `od -c` before their parsers were
// written) - line endings below use plain LF as the closer analogue (both
// cube.txt and mob_drop_item.txt are flat keyword-per-line formats, unlike
// special_item_group.txt's brace-delimited blocks); confirm on first live
// use and switch to CRLF in `serialize` if the real file turns out to need it.
//
// Whitespace/tab-delimited tokens per line (`Cube_load` uses `strtok` with
// " \t\r\n" as delimiters, so any run of spaces/tabs works). A line starting
// with `#` is a full-line comment - not preserved on save, same "regenerate
// wholesale" convention as every other server-file editor in this project.
//
//   section
//   npc <vnum>            (repeatable - which NPC(s) offer this recipe)
//   item <vnum> <count>   (repeatable - required input materials)
//   reward <vnum> <count> (repeatable - output item(s))
//   percent <0-100>       (success chance - a PLAIN 1-100 roll,
//                          `number(1,100) <= percent` in Cube_make, NOT the
//                          mob_drop_item.txt ×10000/÷400 encoding)
//   gold <amount>
//   end                   (commits the recipe - FN_check_cube_data silently
//                          discards it if any npc/item/reward vnum, or any
//                          item/reward count, is 0 - `parse` enforces the
//                          same rule up front so a recipe never gets
//                          uploaded that the server would just drop without
//                          any visible error)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CubeValue {
    pub vnum: u32,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CubeRecipe {
    pub npc_vnums: Vec<u32>,
    pub items: Vec<CubeValue>,
    pub rewards: Vec<CubeValue>,
    pub percent: i32,
    pub gold: i64,
}

/// Deliberately strict, matching this project's other line-oriented server
/// file editors: any line that doesn't match the expected structure fails
/// the whole parse with a line number rather than silently dropping or
/// guessing - the file gets overwritten wholesale on save.
pub fn parse(content: &str) -> Result<Vec<CubeRecipe>, String> {
    let mut recipes = Vec::new();
    let mut current: Option<CubeRecipe> = None;

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();

        match tokens[0] {
            "section" => {
                if current.is_some() {
                    return Err(format!(
                        "Zeile {line_no}: 'section' ohne vorheriges 'end' - verschachtelte Abschnitte werden nicht unterstützt."
                    ));
                }
                current = Some(CubeRecipe { npc_vnums: Vec::new(), items: Vec::new(), rewards: Vec::new(), percent: 0, gold: 0 });
            }
            "npc" => {
                let recipe = current
                    .as_mut()
                    .ok_or_else(|| format!("Zeile {line_no}: 'npc' außerhalb eines 'section'-Blocks."))?;
                let vnum: u32 = tokens
                    .get(1)
                    .ok_or_else(|| format!("Zeile {line_no}: 'npc <vnum>' erwartet."))?
                    .parse()
                    .map_err(|_| format!("Zeile {line_no}: ungültige NPC-VNUM '{}'", tokens[1]))?;
                recipe.npc_vnums.push(vnum);
            }
            "item" => {
                let value = parse_value(&tokens, line_no)?;
                current
                    .as_mut()
                    .ok_or_else(|| format!("Zeile {line_no}: 'item' außerhalb eines 'section'-Blocks."))?
                    .items
                    .push(value);
            }
            "reward" => {
                let value = parse_value(&tokens, line_no)?;
                current
                    .as_mut()
                    .ok_or_else(|| format!("Zeile {line_no}: 'reward' außerhalb eines 'section'-Blocks."))?
                    .rewards
                    .push(value);
            }
            "percent" => {
                let percent: i32 = tokens
                    .get(1)
                    .ok_or_else(|| format!("Zeile {line_no}: 'percent <wert>' erwartet."))?
                    .parse()
                    .map_err(|_| format!("Zeile {line_no}: ungültiger Prozentwert '{}'", tokens[1]))?;
                current
                    .as_mut()
                    .ok_or_else(|| format!("Zeile {line_no}: 'percent' außerhalb eines 'section'-Blocks."))?
                    .percent = percent;
            }
            "gold" => {
                let gold: i64 = tokens
                    .get(1)
                    .ok_or_else(|| format!("Zeile {line_no}: 'gold <wert>' erwartet."))?
                    .parse()
                    .map_err(|_| format!("Zeile {line_no}: ungültiger Gold-Wert '{}'", tokens[1]))?;
                current
                    .as_mut()
                    .ok_or_else(|| format!("Zeile {line_no}: 'gold' außerhalb eines 'section'-Blocks."))?
                    .gold = gold;
            }
            "end" => {
                let recipe = current
                    .take()
                    .ok_or_else(|| format!("Zeile {line_no}: 'end' ohne vorheriges 'section'."))?;
                validate(&recipe, line_no)?;
                recipes.push(recipe);
            }
            other => {
                return Err(format!("Zeile {line_no}: unbekanntes Schlüsselwort '{other}'."));
            }
        }
    }

    if current.is_some() {
        return Err("Datei endet mitten in einem 'section'-Block (fehlendes 'end').".to_string());
    }

    Ok(recipes)
}

fn parse_value(tokens: &[&str], line_no: usize) -> Result<CubeValue, String> {
    let vnum: u32 = tokens
        .get(1)
        .ok_or_else(|| format!("Zeile {line_no}: '<vnum> <anzahl>' erwartet."))?
        .parse()
        .map_err(|_| format!("Zeile {line_no}: ungültige VNUM '{}'", tokens[1]))?;
    let count: i32 = tokens
        .get(2)
        .ok_or_else(|| format!("Zeile {line_no}: '<vnum> <anzahl>' erwartet."))?
        .parse()
        .map_err(|_| format!("Zeile {line_no}: ungültige Anzahl '{}'", tokens[2]))?;
    Ok(CubeValue { vnum, count })
}

fn validate(recipe: &CubeRecipe, end_line_no: usize) -> Result<(), String> {
    if recipe.npc_vnums.iter().any(|&v| v == 0) {
        return Err(format!("Abschnitt (endet Zeile {end_line_no}): eine NPC-VNUM ist 0 - der Server würde dieses Rezept sonst stillschweigend verwerfen."));
    }
    if recipe.items.iter().any(|v| v.vnum == 0 || v.count == 0) {
        return Err(format!("Abschnitt (endet Zeile {end_line_no}): eine Material-VNUM oder -Anzahl ist 0 - der Server würde dieses Rezept sonst stillschweigend verwerfen."));
    }
    if recipe.rewards.iter().any(|v| v.vnum == 0 || v.count == 0) {
        return Err(format!("Abschnitt (endet Zeile {end_line_no}): eine Belohnungs-VNUM oder -Anzahl ist 0 - der Server würde dieses Rezept sonst stillschweigend verwerfen."));
    }
    Ok(())
}

pub fn serialize(recipes: &[CubeRecipe]) -> String {
    let mut out = String::new();
    for recipe in recipes {
        out.push_str("section\n");
        for npc in &recipe.npc_vnums {
            out.push_str(&format!("npc\t{npc}\n"));
        }
        for item in &recipe.items {
            out.push_str(&format!("item\t{}\t{}\n", item.vnum, item.count));
        }
        for reward in &recipe.rewards {
            out.push_str(&format!("reward\t{}\t{}\n", reward.vnum, reward.count));
        }
        out.push_str(&format!("percent\t{}\n", recipe.percent));
        out.push_str(&format!("gold\t{}\n", recipe.gold));
        out.push_str("end\n\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Constructed directly from the verified `Cube_load` grammar above, not
    // a live sample (no real cube.txt was available to byte-verify this
    // session - see the module comment).
    const SAMPLE: &str = "\
# Beispiel-Rezept\nsection\nnpc\t20120\nitem\t1\t2\nitem\t2\t1\nreward\t101\t1\npercent\t50\ngold\t1000\nend\n\nsection\nnpc\t20120\nnpc\t20121\nreward\t102\t1\npercent\t100\ngold\t0\nend\n";

    #[test]
    fn parses_real_grammar() {
        let recipes = parse(SAMPLE).expect("should parse");
        assert_eq!(recipes.len(), 2);
        assert_eq!(recipes[0].npc_vnums, vec![20120]);
        assert_eq!(recipes[0].items, vec![CubeValue { vnum: 1, count: 2 }, CubeValue { vnum: 2, count: 1 }]);
        assert_eq!(recipes[0].rewards, vec![CubeValue { vnum: 101, count: 1 }]);
        assert_eq!(recipes[0].percent, 50);
        assert_eq!(recipes[0].gold, 1000);
        assert_eq!(recipes[1].npc_vnums, vec![20120, 20121]);
    }

    #[test]
    fn round_trips() {
        let recipes = parse(SAMPLE).unwrap();
        let rendered = serialize(&recipes);
        let reparsed = parse(&rendered).unwrap();
        assert_eq!(recipes, reparsed);
    }

    #[test]
    fn rejects_zero_npc_vnum_like_the_server_would_silently_drop_it() {
        let content = "section\nnpc\t0\nitem\t1\t1\nreward\t2\t1\npercent\t10\ngold\t0\nend\n";
        let err = parse(content).unwrap_err();
        assert!(err.contains("NPC-VNUM"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_zero_item_count() {
        let content = "section\nnpc\t1\nitem\t5\t0\nreward\t2\t1\npercent\t10\ngold\t0\nend\n";
        assert!(parse(content).is_err());
    }

    #[test]
    fn rejects_end_without_section() {
        assert!(parse("end\n").is_err());
    }

    #[test]
    fn rejects_nested_section() {
        assert!(parse("section\nsection\nend\n").is_err());
    }

    #[test]
    fn rejects_unclosed_section() {
        assert!(parse("section\nnpc\t1\n").is_err());
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let content = "# comment\n\nsection\nnpc\t1\nitem\t1\t1\nreward\t2\t1\npercent\t10\ngold\t0\nend\n";
        assert_eq!(parse(content).unwrap().len(), 1);
    }
}
