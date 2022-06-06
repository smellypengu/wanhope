use crate::{
    app::AppError,
    egui::{Props, Ui, View},
};

pub struct Chat {
    pub text: String,
    pub messages: Vec<String>,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            text: "".to_string(),
            messages: Vec::new(),
        }
    }
}

impl Ui for Chat {
    fn name(&self) -> &'static str {
        "Chat"
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        open: &mut bool,
        props: &Props,
    ) -> Option<egui::InnerResponse<Option<anyhow::Result<(), AppError>>>> {
        egui::Window::new("Chat")
            .open(open)
            .resizable(false)
            .show(ctx, |ui| -> anyhow::Result<(), AppError> {
                Ok(self.ui(ui, props)?)
            })
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl View for Chat {
    fn ui(&mut self, ui: &mut egui::Ui, props: &Props) -> anyhow::Result<(), AppError> {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(200.0)
            .stick_to_bottom()
            .show(ui, |ui| {
                for message in &self.messages {
                    ui.label(message);
                }
            });

        ui.horizontal(|ui| -> anyhow::Result<(), AppError> {
            ui.text_edit_singleline(&mut self.text);
            if ui.button("Send").clicked() {
                props.network.send_chat_message(&self.text)?;
                self.text = "".to_string();
            };

            Ok(())
        })
        .inner?;

        Ok(())
    }
}
