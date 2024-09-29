mod graphing_engine;

use winit::{
    event::*, event_loop::EventLoop, window::WindowBuilder
};
use graphing_engine::State;

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = State::new(&window).await;

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => if !state.input(event) {
                match &event {
                    WindowEvent::Resized(physical_size) => state.resize(*physical_size),
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::RedrawRequested => {
                        state.window().request_redraw();
                        state.update();

                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size()),
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            Err(e) => eprintln!("{:?}", e),
                        }
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }).unwrap();
}

fn main() {
    pollster::block_on(run());
}
