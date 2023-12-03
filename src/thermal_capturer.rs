use std::{
    mem,
    sync::{mpsc, Arc},
    thread,
};

use eframe::epaint::ColorImage;
use nokhwa::Camera;

use crate::{
    auto_display_range_controller::AutoDisplayRangeController,
    camera_adapter::{infiray_p2_pro::InfirayP2ProAdapter, CameraAdapter},
    temperature::{Temp, TempRange, TemperatureUnit},
    thermal_data::ThermalDataHistogram,
    thermal_gradient::{ThermalGradient, THERMAL_GRADIENTS},
};

pub struct ThermalCapturerResult {
    pub image: ColorImage,
    pub image_range: TempRange,
    pub real_fps: f32,
    pub reported_fps: f32,
    pub histogram: ThermalDataHistogram,
}

#[derive(Clone)]
pub struct ThermalCapturerSettings {
    pub auto_range: bool,
    pub manual_range: TempRange,
    pub gradient: ThermalGradient,
}

pub type ThermalCapturerCallback = Arc<dyn Fn() + Send + Sync>;

enum ThermalCapturerCmd {
    SetSettings(ThermalCapturerSettings),
    Stop,
}

struct ThermalCapturerCtx {
    camera: Camera,
    callback: ThermalCapturerCallback,
    cmd_receiver: mpsc::Receiver<ThermalCapturerCmd>,
    result_sender: mpsc::Sender<Box<ThermalCapturerResult>>,
    adapter: Arc<dyn CameraAdapter>,
    settings: ThermalCapturerSettings,
    auto_range_controller: AutoDisplayRangeController,
}

pub struct ThermalCapturer {
    ctx: Option<ThermalCapturerCtx>,
    cmd_sender: mpsc::Sender<ThermalCapturerCmd>,

    pub result_receiver: mpsc::Receiver<Box<ThermalCapturerResult>>,
}

///
/// ThermalCapturer runs in a background thread continuously capturing images from the camera,
/// And calling the callback function with the captured image.
impl ThermalCapturer {
    pub fn new(
        camera: Camera,
        adapter: Arc<dyn CameraAdapter>,
        callback: ThermalCapturerCallback,
    ) -> Self {
        let (cmd_sender, cmd_receiver) = mpsc::channel();
        let (result_sender, result_receiver) = mpsc::channel();
        Self {
            ctx: Some(ThermalCapturerCtx {
                camera,
                adapter,
                callback,
                cmd_receiver,
                result_sender,
                settings: ThermalCapturerSettings {
                    auto_range: true,
                    manual_range: TempRange::new(
                        Temp::from_unit(TemperatureUnit::Celsius, 0.0),
                        Temp::from_unit(TemperatureUnit::Celsius, 100.0),
                    ),
                    gradient: THERMAL_GRADIENTS[0].clone(),
                },
                auto_range_controller: AutoDisplayRangeController::new(),
            }),
            cmd_sender,
            result_receiver,
        }
    }

    //
    pub fn start(&mut self) {
        // move the camera out of self so we can use it into the thread
        let mut ctx = mem::replace(&mut self.ctx, None).unwrap();
        thread::spawn(move || {
            ctx.camera.open_stream().unwrap();

            let mut last_frame_time;
            loop {
                last_frame_time = std::time::Instant::now();

                let thermal_data = ctx.adapter.capture_thermal_data(&mut ctx.camera).unwrap();

                let (mintemp_pos, maxtemp_pos) = thermal_data.get_min_max_pos();

                let captured_range = TempRange::new(
                    thermal_data.temperature_at(mintemp_pos.x, mintemp_pos.y),
                    thermal_data.temperature_at(maxtemp_pos.x, maxtemp_pos.y),
                );

                let mut mapping_range = ctx.auto_range_controller.compute(captured_range);

                if !ctx.settings.auto_range {
                    mapping_range = ctx.settings.manual_range;
                }

                let image = thermal_data.map_to_image(|temp| {
                    ctx.settings.gradient.get_color(mapping_range.factor(temp))
                });
                let result = Box::new(ThermalCapturerResult {
                    image,
                    real_fps: 1.0 / last_frame_time.elapsed().as_secs_f32(),
                    reported_fps: ctx.camera.frame_rate() as f32,
                    image_range: mapping_range,
                    histogram: ThermalDataHistogram::from_thermal_data(
                        &thermal_data,
                        captured_range.join(mapping_range),
                        100,
                    ),
                });
                ctx.result_sender.send(result).unwrap();
                (ctx.callback)();
                match ctx.cmd_receiver.try_recv() {
                    Ok(cmd) => match cmd {
                        ThermalCapturerCmd::Stop => {
                            ctx.camera.stop_stream().unwrap();
                            break;
                        }
                        ThermalCapturerCmd::SetSettings(range_settings) => {
                            ctx.settings = range_settings;
                        }
                    },
                    Err(_) => {}
                }
            }
        });
    }
    pub fn set_settings(&mut self, settings: ThermalCapturerSettings) {
        self.cmd_sender
            .send(ThermalCapturerCmd::SetSettings(settings))
            .unwrap();
    }
}

impl Drop for ThermalCapturer {
    fn drop(&mut self) {
        self.cmd_sender.send(ThermalCapturerCmd::Stop).unwrap();
    }
}
