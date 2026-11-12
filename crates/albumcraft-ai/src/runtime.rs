//! Running a network: an ONNX file loaded once, then called with tensors.
//!
//! ONNX Runtime is linked into the program when the `onnx` feature is on (the default), except
//! on Windows with the GNU toolchain, for which no prebuilt library exists. Where it is missing,
//! `available()` answers false and `Network::load` fails with a message; nothing else changes.
//! The `ort` types stay inside this file.

use std::path::Path;

/// A block of numbers going into or coming out of a network.
#[derive(Clone, Debug, PartialEq)]
pub enum Tensor {
    F32 { shape: Vec<usize>, data: Vec<f32> },
    /// Whole numbers (the word numbers of a text encoder).
    I64 { shape: Vec<usize>, data: Vec<i64> },
}

impl Tensor {
    pub fn shape(&self) -> &[usize] {
        match self {
            Tensor::F32 { shape, .. } | Tensor::I64 { shape, .. } => shape,
        }
    }

    /// The numbers of an `F32` tensor.
    pub fn floats(&self) -> Option<&[f32]> {
        match self {
            Tensor::F32 { data, .. } => Some(data),
            Tensor::I64 { .. } => None,
        }
    }

    /// The shape and the number of values agree.
    fn consistent(&self) -> bool {
        let n = self.shape().iter().try_fold(1usize, |n, &d| n.checked_mul(d));
        let len = match self {
            Tensor::F32 { data, .. } => data.len(),
            Tensor::I64 { data, .. } => data.len(),
        };
        n == Some(len)
    }
}

/// An input or output of a network: its name and shape (-1 where the size is free).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Port {
    pub name: String,
    pub shape: Vec<i64>,
}

/// A loaded network. Calls from several threads wait for each other.
pub struct Network(imp::Network);

impl Network {
    /// Load a network file. It computes on one thread: analysis runs in the background and must
    /// leave the computer free for the window.
    pub fn load(path: &Path) -> Result<Network, String> {
        Network::load_threads(path, 1)
    }

    /// Load a network that computes on up to `threads` threads (the background analysis has
    /// two; calls wait for each other, so a second worker would otherwise sit idle).
    pub fn load_threads(path: &Path, threads: usize) -> Result<Network, String> {
        imp::Network::load(path, threads.clamp(1, 8)).map(Network)
    }

    pub fn inputs(&self) -> &[Port] {
        &self.0.inputs
    }

    pub fn outputs(&self) -> &[Port] {
        &self.0.outputs
    }

    /// Run it on the named inputs; the outputs come back in the order of `outputs()`.
    pub fn run(&self, inputs: &[(&str, Tensor)]) -> Result<Vec<Tensor>, String> {
        if let Some((name, _)) = inputs.iter().find(|(_, t)| !t.consistent()) {
            return Err(format!("the input {name} has a shape that does not match its values"));
        }
        self.0.run(inputs)
    }
}

/// Networks can be run by this build of the program.
pub fn available() -> bool {
    imp::AVAILABLE
}

#[cfg(all(feature = "onnx", not(all(windows, target_env = "gnu"))))]
mod imp {
    use super::{Port, Tensor};
    use ort::session::{Session, SessionInputValue};
    use ort::value::{Outlet, ValueType};
    use std::path::Path;
    use std::sync::{Mutex, OnceLock, PoisonError};

    pub const AVAILABLE: bool = true;

    pub struct Network {
        session: Mutex<Session>,
        pub inputs: Vec<Port>,
        pub outputs: Vec<Port>,
    }

    /// The one ONNX Runtime environment of the program (it sends no usage data anywhere).
    fn environment() -> Result<&'static ort::environment::Environment, String> {
        static ENV: OnceLock<Result<ort::environment::Environment, String>> = OnceLock::new();
        ENV.get_or_init(|| ort::init().with_name("albumcraft").with_telemetry(false).build().map_err(|e| e.to_string())).as_ref().map_err(String::clone)
    }

    fn ports(outlets: &[Outlet]) -> Vec<Port> {
        outlets
            .iter()
            .map(|o| Port {
                name: o.name().to_string(),
                shape: match o.dtype() {
                    ValueType::Tensor { shape, .. } => shape.to_vec(),
                    _ => vec![],
                },
            })
            .collect()
    }

    fn dims(shape: &[usize]) -> Vec<i64> {
        shape.iter().map(|&d| d as i64).collect()
    }

    impl Network {
        pub fn load(path: &Path, threads: usize) -> Result<Network, String> {
            let failed = |e: String| format!("the network {} could not be loaded: {e}", path.display());
            // a damaged file must give a message, whatever the library does with it
            let loaded = std::panic::catch_unwind(|| -> Result<Session, String> {
                let mut builder = Session::builder(environment()?).map_err(|e| e.to_string())?.with_intra_threads(threads).map_err(|e| e.to_string())?;
                builder.commit_from_file(path).map_err(|e| e.to_string())
            });
            let session = loaded.unwrap_or_else(|_| Err("unexpected error".into())).map_err(failed)?;
            let (inputs, outputs) = (ports(session.inputs()), ports(session.outputs()));
            Ok(Network { session: Mutex::new(session), inputs, outputs })
        }

        pub fn run(&self, inputs: &[(&str, Tensor)]) -> Result<Vec<Tensor>, String> {
            let mut values: Vec<(String, SessionInputValue<'_>)> = Vec::with_capacity(inputs.len());
            for (name, tensor) in inputs {
                let value: SessionInputValue<'_> = match tensor {
                    Tensor::F32 { shape, data } => ort::value::Tensor::from_array((dims(shape), data.clone())).map_err(|e| e.to_string())?.into(),
                    Tensor::I64 { shape, data } => ort::value::Tensor::from_array((dims(shape), data.clone())).map_err(|e| e.to_string())?.into(),
                };
                values.push((name.to_string(), value));
            }
            let mut session = self.session.lock().unwrap_or_else(PoisonError::into_inner);
            let found = session.run(values).map_err(|e| e.to_string())?;
            let mut out = Vec::with_capacity(self.outputs.len());
            for port in &self.outputs {
                let value = found.get(&port.name).ok_or_else(|| format!("the network gave no {}", port.name))?;
                let shape = |s: &[i64]| s.iter().map(|&d| d.max(0) as usize).collect();
                if let Ok((s, data)) = value.try_extract_tensor::<f32>() {
                    out.push(Tensor::F32 { shape: shape(s), data: data.to_vec() });
                } else if let Ok((s, data)) = value.try_extract_tensor::<i64>() {
                    out.push(Tensor::I64 { shape: shape(s), data: data.to_vec() });
                } else {
                    return Err(format!("the output {} is of a kind that is not handled", port.name));
                }
            }
            Ok(out)
        }
    }
}

#[cfg(not(all(feature = "onnx", not(all(windows, target_env = "gnu")))))]
mod imp {
    use super::{Port, Tensor};
    use std::path::Path;

    pub const AVAILABLE: bool = false;
    const MISSING: &str = "this build of AlbumCraft cannot run networks";

    pub struct Network {
        pub inputs: Vec<Port>,
        pub outputs: Vec<Port>,
    }

    impl Network {
        pub fn load(_: &Path, _: usize) -> Result<Network, String> {
            Err(MISSING.into())
        }

        pub fn run(&self, _: &[(&str, Tensor)]) -> Result<Vec<Tensor>, String> {
            Err(MISSING.into())
        }
    }
}

/// A tiny network written out by hand, for the tests: `y = x + float(n)`, with `x` four
/// numbers and `n` four whole numbers.
#[cfg(test)]
pub(crate) fn tiny_network() -> Vec<u8> {
    // the few protocol buffer pieces an ONNX file is made of
    fn varint(mut v: u64, out: &mut Vec<u8>) {
        while v >= 0x80 {
            out.push((v as u8 & 0x7f) | 0x80);
            v >>= 7;
        }
        out.push(v as u8);
    }
    fn int(field: u64, v: u64) -> Vec<u8> {
        let mut out = vec![];
        varint(field << 3, &mut out);
        varint(v, &mut out);
        out
    }
    fn bytes(field: u64, b: &[u8]) -> Vec<u8> {
        let mut out = vec![];
        varint(field << 3 | 2, &mut out);
        varint(b.len() as u64, &mut out);
        out.extend_from_slice(b);
        out
    }
    fn text(field: u64, s: &str) -> Vec<u8> {
        bytes(field, s.as_bytes())
    }
    // a [1, 4] tensor of element type `elem` named `name`
    fn value(name: &str, elem: u64) -> Vec<u8> {
        let dim = |n| bytes(1, &int(1, n));
        let shape = [dim(1), dim(4)].concat();
        let tensor = [int(1, elem), bytes(2, &shape)].concat();
        [text(1, name), bytes(2, &bytes(1, &tensor))].concat()
    }
    const FLOAT: u64 = 1;
    const INT64: u64 = 7;
    let to_float = [text(1, "to"), int(3, FLOAT), int(20, 2)].concat();
    let cast = [text(1, "n"), text(2, "nf"), text(4, "Cast"), bytes(5, &to_float)].concat();
    let add = [text(1, "x"), text(1, "nf"), text(2, "y"), text(4, "Add")].concat();
    let graph = [bytes(1, &cast), bytes(1, &add), text(2, "tiny"), bytes(11, &value("x", FLOAT)), bytes(11, &value("n", INT64)), bytes(12, &value("y", FLOAT))].concat();
    let opset = [text(1, ""), int(2, 13)].concat();
    [int(1, 8), bytes(8, &opset), bytes(7, &graph)].concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("albumcraft-ai-runtime-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// ONNX Runtime is really linked in and computes: the check that the three-OS build works.
    #[test]
    fn a_network_runs() {
        if !available() {
            return;
        }
        let dir = temp("runs");
        let file = dir.join("tiny.onnx");
        std::fs::write(&file, tiny_network()).unwrap();
        let net = Network::load(&file).unwrap();
        assert_eq!(net.inputs(), [Port { name: "x".into(), shape: vec![1, 4] }, Port { name: "n".into(), shape: vec![1, 4] }]);
        assert_eq!(net.outputs(), [Port { name: "y".into(), shape: vec![1, 4] }]);
        let x = Tensor::F32 { shape: vec![1, 4], data: vec![0.5, 1.5, 2.5, 3.5] };
        let n = Tensor::I64 { shape: vec![1, 4], data: vec![10, 20, 30, 40] };
        let out = net.run(&[("x", x.clone()), ("n", n.clone())]).unwrap();
        assert_eq!(out, [Tensor::F32 { shape: vec![1, 4], data: vec![10.5, 21.5, 32.5, 43.5] }]);
        // from several threads at once
        let net = std::sync::Arc::new(net);
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let (net, x, n) = (net.clone(), x.clone(), n.clone());
                std::thread::spawn(move || net.run(&[("x", x), ("n", n)]).unwrap()[0].floats().unwrap()[3])
            })
            .collect();
        for t in threads {
            assert_eq!(t.join().unwrap(), 43.5);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_files_and_bad_inputs_are_errors_not_crashes() {
        let dir = temp("bad");
        let missing = Network::load(&dir.join("none.onnx"));
        assert!(missing.is_err());
        let file = dir.join("damaged.onnx");
        std::fs::write(&file, b"this is not a network").unwrap();
        assert!(Network::load(&file).is_err_and(|e| e.contains("damaged.onnx") || !available()));
        if available() {
            std::fs::write(&file, tiny_network()).unwrap();
            let net = Network::load(&file).unwrap();
            let x = Tensor::F32 { shape: vec![1, 4], data: vec![0.0; 4] };
            // a missing input, a wrong shape, a shape that does not match the values
            assert!(net.run(&[("x", x.clone())]).is_err());
            let short = Tensor::I64 { shape: vec![1, 2], data: vec![1, 2] };
            assert!(net.run(&[("x", x.clone()), ("n", short)]).is_err());
            let lying = Tensor::I64 { shape: vec![1, 4], data: vec![1] };
            assert!(net.run(&[("x", x), ("n", lying)]).is_err_and(|e| e.contains("shape")));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
