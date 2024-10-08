use std::{marker::PhantomData, sync::Arc, task::Wake};

use eframe::{run_native, App, CreationContext, Frame, NativeOptions, Result};
use egui::Context;

pub fn run_simple_native(
    app_name: &str,
    native_options: NativeOptions,
    update_fun: impl FnMut(&Context, &mut Frame) + 'static,
) -> Result<()> {
    run_native(
        app_name,
        native_options,
        Box::new(|ctx| {
            let rt = RtLocalRuntime::new(ctx);
            let update = Box::new(update_fun);
            Ok(Box::new(SimpleApp { rt, update }))
        }),
    )?;
    Ok(())
}

#[allow(clippy::type_complexity)]
struct SimpleApp {
    rt: RtLocalRuntime,
    update: Box<dyn FnMut(&Context, &mut Frame)>,
}

impl App for SimpleApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.rt.before_update();
        (self.update)(ctx, frame);
        self.rt.after_update();
    }
}
impl Drop for SimpleApp {
    fn drop(&mut self) {}
}

#[derive(Default)]
struct PhantomNotSend(PhantomData<*mut ()>);

pub struct RtLocalRuntime {
    ctx: Context,
    _not_send: PhantomNotSend,
}

impl RtLocalRuntime {
    pub fn new(ctx: &CreationContext) -> Self {
        rt_local_core::base::enter(Arc::new(EguiWake(ctx.egui_ctx.clone())).into());
        Self {
            ctx: ctx.egui_ctx.clone(),
            _not_send: PhantomNotSend::default(),
        }
    }
    pub fn before_update(&self) {
        rt_local_core::base::poll();
    }
    pub fn after_update(&self) {
        if self.ctx.has_requested_repaint() {
            rt_local_core::base::idle();
        }
    }
}
impl Drop for RtLocalRuntime {
    fn drop(&mut self) {
        rt_local_core::base::leave();
    }
}

struct EguiWake(Context);

impl Wake for EguiWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.request_repaint();
    }
}
