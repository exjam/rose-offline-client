use bevy_egui::egui;

pub trait DrawText {
    fn add_at(&mut self, pos: egui::Pos2, widget: impl egui::Widget) -> egui::Response;

    fn add_label_at(
        &mut self,
        pos: egui::Pos2,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response;

    fn add_label_in(
        &mut self,
        rect: egui::Rect,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response;
}

impl DrawText for egui::Ui {
    fn add_at(&mut self, pos: egui::Pos2, widget: impl egui::Widget) -> egui::Response {
        let mut rect = self.min_rect();
        rect.min += pos.to_vec2();
        self.allocate_ui_at_rect(rect, |ui| ui.horizontal_top(|ui| ui.add(widget)).inner)
            .inner
    }

    fn add_label_at(
        &mut self,
        pos: egui::Pos2,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response {
        self.add_at(pos, egui::Label::new(text))
    }

    fn add_label_in(
        &mut self,
        rect: egui::Rect,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response {
        self.put(
            rect.translate(self.min_rect().min.to_vec2()),
            egui::Label::new(text).wrap(true),
        )
    }
}
