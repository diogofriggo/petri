// let path = r"/home/diogo/gounizar/redes/Practica2/MiniproyectoSD/CodigoSuministradoAAlumnos/simuladores/distconssim/testdata/3subredes.subred0.json";
// let path = r"/home/diogo/gounizar/redes/Practica2/MiniproyectoSD/CodigoSuministradoAAlumnos/simuladores/distconssim/testdata/3subredes.subred1.json";
// let path = r"/home/diogo/gounizar/redes/Practica2/MiniproyectoSD/CodigoSuministradoAAlumnos/simuladores/distconssim/testdata/3subredes.subred2.json";
mod engine;
mod error;
mod json;
mod model;
mod tcp;

use error::Result;

use crate::engine::Engine;

fn main() -> Result<()> {
    // TODO: CLI inputs
    let last_clock = 3;
    let node = "127.0.0.1:1";
    let nodes = ["127.0.0.1:1", "127.0.0.1:2", "127.0.0.1:3"];
    let nets_folder = "/home/diogo/gounizar/redes/Practica2/MiniproyectoSD/CodigoSuministradoAAlumnos/simuladores/distconssim";
    let mut engine = Engine::new(last_clock, node.into(), &nodes, nets_folder)?;
    engine.run()
}
