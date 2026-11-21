//! Gemeinsame Rückfrage "Ungespeicherte Änderungen verwerfen?". Nutzt den
//! nativen Bestätigungsdialog (blockiert alles andere, funktioniert auch
//! in async-Abläufen wie dem Serverwechsel oder dem Schließen des Fensters).

use crate::store::navigation::nav;
use crate::tauri::dialog_confirm;
use leptos::task::spawn_local;

const TITLE: &str = "Ungespeicherte Änderungen";

/// Fragt nach und ruft `on_confirm` nur nach Zustimmung auf.
pub fn confirm_discard(message: &str, on_confirm: impl FnOnce() + 'static) {
    let message = message.to_string();
    spawn_local(async move {
        if dialog_confirm(&message, Some(TITLE)).await {
            on_confirm();
        }
    });
}

/// Wie [`confirm_discard`], fragt aber nur, wenn `dirty` wahr ist; sonst
/// wird `on_confirm` sofort ausgeführt.
pub fn confirm_discard_if(dirty: bool, message: &str, on_confirm: impl FnOnce() + 'static) {
    if dirty {
        confirm_discard(message, on_confirm);
    } else {
        on_confirm();
    }
}

/// Rückfrage vor einer Aktion, die alle Bereiche betrifft (Serverwechsel,
/// Fenster schließen). `true`, wenn nichts ungespeichert ist oder zugestimmt wurde.
pub async fn confirm_discard_all(action: &str) -> bool {
    let sections = nav().dirty_sections();
    if sections.is_empty() {
        return true;
    }
    let names: Vec<String> = sections.iter().map(|s| crate::i18n::t(s.label_key())).collect();
    dialog_confirm(&discard_all_message(action, &names), Some(TITLE)).await
}

pub fn discard_all_message(action: &str, names: &[String]) -> String {
    format!("Es gibt ungespeicherte Änderungen ({}). {action} und die Änderungen verwerfen?", names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_lists_sections() {
        let m = discard_all_message("Server wechseln", &["Item Editor".into(), "Cube".into()]);
        assert!(m.contains("(Item Editor, Cube)"));
        assert!(m.contains("Server wechseln und"));
    }
}
