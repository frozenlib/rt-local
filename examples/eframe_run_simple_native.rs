use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use eframe::{NativeOptions, Result};
use egui::{Label, ProgressBar};
use rt_local::{runtime::eframe::run_simple_native, spawn_local};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Using `block_in_place` is necessary, otherwise,
    // if the budget is exhausted, Tokio's asynchronous functions will always return `Pending`, causing problems.
    tokio::task::block_in_place(|| {
        let options = NativeOptions::default();
        let p = Rc::new(Cell::new(0.0));
        let text = Rc::new(RefCell::new(String::new()));
        let mut task = None;
        run_simple_native("simple native", options, move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| {
                if ui.button("spawn").clicked() {
                    let p = p.clone();
                    let text = text.clone();
                    p.set(0.0);
                    task = Some(spawn_local(async move {
                        for _ in 0..200 {
                            p.set(p.get() + 0.005);
                            *text.borrow_mut() = format!("{:.2}", p.get());
                            sleep(Duration::from_millis(10)).await;
                        }
                        *text.borrow_mut() = "Done".to_string();
                    }));
                }
                ui.add(ProgressBar::new(p.get()));
                ui.add(Label::new(text.borrow().as_str()));
            });
        })
    })
}
