use eldenring::cs::{CSFD4FadePlate, CSFD4FadePlateColor, CSFade, FadePlateEasing, FadePlateId};
use hudhook::imgui::Ui;

use debug::UiExt;

use super::{DisplayUiExt, StatefulDebugDisplay};

const FADE_PLATE_IDS: [FadePlateId; 9] = [
    FadePlateId::Title,
    FadePlateId::MapIn,
    FadePlateId::InGame,
    FadePlateId::Cutscene,
    FadePlateId::InCutscene,
    FadePlateId::Ending,
    FadePlateId::Event,
    FadePlateId::InGameChroma,
    FadePlateId::InGameBokeh,
];

const EASING_MODES: [FadePlateEasing; 3] = [
    FadePlateEasing::Linear,
    FadePlateEasing::EaseIn,
    FadePlateEasing::EaseOut,
];

impl StatefulDebugDisplay for CSFade {
    type State = ();

    fn render_debug_mut(&mut self, ui: &Ui, _state: &mut Self::State) {
        ui.display(
            "Combined fade color",
            format!("{:?}", self.get_combined_fade_color()),
        );

        ui.input_float("Bokeh blend multiplier", &mut self.bokeh_blend_multiplier)
            .build();

        ui.list(
            "Fade plates",
            self.fade_plates.iter_mut().zip(FADE_PLATE_IDS),
            |ui, _i, (plate, id)| {
                ui.nested_mut(format!("{id:?}"), &mut **plate, &mut ());
            },
        );
    }
}

impl StatefulDebugDisplay for CSFD4FadePlate {
    type State = ();

    fn render_debug_mut(&mut self, ui: &Ui, _state: &mut Self::State) {
        ui.display("Title", &self.title);
        ui.display("Effective color", format!("{:?}", self.effective_color()));

        let mut current_color: [f32; 4] = (&self.current_color).into();
        if ui.color_edit4("Current color", &mut current_color) {
            self.current_color = current_color.into();
        }

        let mut transition_color: [f32; 4] = (&self.transition_color).into();
        if ui.color_edit4("Transition color", &mut transition_color) {
            self.transition_color = transition_color.into();
        }

        let mut target_color: [f32; 4] = (&self.target_color).into();
        if ui.color_edit4("Target color", &mut target_color) {
            self.target_color = target_color.into();
        }

        ui.input_float("Fade timer", &mut self.fade_timer.time)
            .build();
        ui.input_float("Fade duration", &mut self.fade_duration.time)
            .build();

        let mut easing_index = EASING_MODES
            .iter()
            .position(|&mode| mode == self.easing)
            .unwrap_or(0);
        if ui.combo("Easing", &mut easing_index, &EASING_MODES, |mode| {
            format!("{mode:?}").into()
        }) {
            self.easing = EASING_MODES[easing_index];
        }

        ui.header("Force flags", || {
            let mut interpolation_paused = self.force_flags.interpolation_paused();
            if ui.checkbox("Interpolation paused", &mut interpolation_paused) {
                self.force_flags
                    .set_interpolation_paused(interpolation_paused);
            }

            let mut force_cleared = self.force_flags.force_cleared();
            if ui.checkbox("Force cleared", &mut force_cleared) {
                self.force_flags.set_force_cleared(force_cleared);
            }

            let mut debug_interpolation_paused = self.force_flags.debug_interpolation_paused();
            if ui.checkbox(
                "Debug interpolation paused",
                &mut debug_interpolation_paused,
            ) {
                self.force_flags
                    .set_debug_interpolation_paused(debug_interpolation_paused);
            }

            let mut debug_force_cleared = self.force_flags.debug_force_cleared();
            if ui.checkbox("Debug force cleared", &mut debug_force_cleared) {
                self.force_flags
                    .debug_set_force_cleared(debug_force_cleared);
            }
        });

        if ui.button("Snap to current color") {
            let color: CSFD4FadePlateColor = current_color.into();
            self.set_color_instant(color);
        }
    }
}
