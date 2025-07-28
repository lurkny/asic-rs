use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BitaxeModel {
    Supra,
    Gamma,
    Max,
    Ultra,
}
