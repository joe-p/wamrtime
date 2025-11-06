use crate::Result;
use color_eyre::eyre::eyre;

use radix_wasm_instrument::{
    gas_metering::{ConstantCostRules, host_function, inject},
    utils::module_info::ModuleInfo,
};

pub struct Compiler {
    rules: ConstantCostRules,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            rules: ConstantCostRules::new(1, 10_000, 1),
        }
    }

    pub fn compile_wasm(self, raw_wasm_bytes: &mut [u8]) -> Result<Vec<u8>> {
        let backend = host_function::Injector::new("env", "host_gas_check");

        let mut module = ModuleInfo::new(raw_wasm_bytes)
            .map_err(|err| eyre!("Failed to create ModuleInfo from bytes: {err}"))?;

        let wasm_bytes = inject(&mut module, backend, &self.rules)
            .map_err(|err| eyre!("Failed to inject gas metering: {err}"))?;

        Ok(wasm_bytes.to_vec())
    }
}
