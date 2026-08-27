use hudhook::imgui::{TableColumnSetup, Ui};

use debug::UiExt;
use eldenring::cs::{MSG_TAG_COUNT, MsgTagManImp};

use super::DebugDisplay;

impl DebugDisplay for MsgTagManImp {
    fn render_debug(&self, ui: &Ui) {
        ui.text("Substitutions applied to menu text containing <?name?>.");

        let named = (0..MSG_TAG_COUNT).filter(|index| self.name(*index).is_some());

        ui.table(
            "message-tags",
            [
                TableColumnSetup::new("Index"),
                TableColumnSetup::new("Name"),
                TableColumnSetup::new("Value"),
                TableColumnSetup::new("Capacity"),
                TableColumnSetup::new("Callback"),
            ],
            named,
            |ui, _, index: usize| {
                let tag = &self.tags[index];

                ui.table_next_column();
                ui.text(index.to_string());

                ui.table_next_column();
                ui.text(
                    self.name(index)
                        .map(String::from_utf16_lossy)
                        .unwrap_or_default(),
                );

                ui.table_next_column();
                ui.text(
                    self.value(index)
                        .map(String::from_utf16_lossy)
                        .unwrap_or_default(),
                );

                ui.table_next_column();
                ui.text(tag.capacity.to_string());

                ui.table_next_column();
                ui.text(if tag.callback.is_some() { "yes" } else { "no" });
            },
        );
    }
}
