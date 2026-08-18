# Architecture Documentation

The canonical implementation overview is [`../../ARCHITECTURE.md`](../../ARCHITECTURE.md).
It describes the current Cargo workspace, parser/model/resolver/renderer pipeline, hierarchy resolution, and public API surface.

Use the focused documents below for contracts that change more frequently than the module map:

- [`CAPABILITY_MATRIX.md`](./CAPABILITY_MATRIX.md) - authoritative support stages for all 56 bounded semantic capabilities.
- [`PPTX_COMPLETENESS_CONTRACT.md`](./PPTX_COMPLETENESS_CONTRACT.md) - exactness rules, diagnostics, and evidence requirements.
- [`PPTX_COMPLETENESS_PROGRESS.md`](./PPTX_COMPLETENESS_PROGRESS.md) - current completion state and validation gates.
- [`REMAINING_WORK_PLAN.md`](./REMAINING_WORK_PLAN.md) - remaining PowerPoint-native exactness work.
- [`SUPPORTED_FEATURES.md`](./SUPPORTED_FEATURES.md) - pointer to the generated ECMA-376 support inventory.

The generated capability sections in these documents are synchronized from `evaluate/completeness_manifest.json` by `evaluate/check_exactness_contract.py`.
