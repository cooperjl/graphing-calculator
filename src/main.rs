mod graphing_engine;

use graphing_engine::run;

fn main() {
    pollster::block_on(run());
}
