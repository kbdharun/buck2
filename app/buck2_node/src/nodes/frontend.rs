/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use buck2_core::package::PackageLabel;
use buck2_core::target::label::TargetLabel;
use buck2_util::late_binding::LateBinding;
use dice::DiceComputations;
use dupe::Dupe;

use crate::nodes::eval_result::EvaluationResult;
use crate::nodes::unconfigured::TargetNode;

#[async_trait]
pub trait TargetGraphCalculationImpl: Send + Sync + 'static {
    /// Like `get_interpreter_results` but doesn't cache the result on the DICE graph.
    async fn get_interpreter_results_uncached(
        &self,
        ctx: &DiceComputations,
        package: PackageLabel,
    ) -> anyhow::Result<Arc<EvaluationResult>>;

    /// Returns the full interpreter evaluation result for a Package. This consists of the full set
    /// of `TargetNode`s of interpreting that build file.
    async fn get_interpreter_results(
        &self,
        ctx: &DiceComputations,
        package: PackageLabel,
    ) -> anyhow::Result<Arc<EvaluationResult>>;
}

pub static TARGET_GRAPH_CALCULATION_IMPL: LateBinding<&'static dyn TargetGraphCalculationImpl> =
    LateBinding::new("TARGET_GRAPH_CALCULATION_IMPL");

#[async_trait]
pub trait TargetGraphCalculation {
    /// Like `get_interpreter_results` but doesn't cache the result on the DICE graph.
    async fn get_interpreter_results_uncached(
        &self,
        package: PackageLabel,
    ) -> anyhow::Result<Arc<EvaluationResult>>;

    /// Returns the full interpreter evaluation result for a Package. This consists of the full set
    /// of `TargetNode`s of interpreting that build file.
    async fn get_interpreter_results(
        &self,
        package: PackageLabel,
    ) -> anyhow::Result<Arc<EvaluationResult>>;

    /// For a TargetLabel, returns the TargetNode. This is really just part of the the interpreter
    /// results for the the label's package, and so this is just a utility for accessing that, it
    /// isn't separately cached.
    async fn get_target_node(&self, target: &TargetLabel) -> anyhow::Result<TargetNode>;
}

#[async_trait]
impl TargetGraphCalculation for DiceComputations {
    async fn get_interpreter_results_uncached(
        &self,
        package: PackageLabel,
    ) -> anyhow::Result<Arc<EvaluationResult>> {
        TARGET_GRAPH_CALCULATION_IMPL
            .get()?
            .get_interpreter_results_uncached(self, package)
            .await
    }

    async fn get_interpreter_results(
        &self,
        package: PackageLabel,
    ) -> anyhow::Result<Arc<EvaluationResult>> {
        TARGET_GRAPH_CALCULATION_IMPL
            .get()?
            .get_interpreter_results(self, package)
            .await
    }

    async fn get_target_node(&self, target: &TargetLabel) -> anyhow::Result<TargetNode> {
        Ok(TARGET_GRAPH_CALCULATION_IMPL
            .get()?
            .get_interpreter_results(self, target.pkg())
            .await
            .with_context(|| {
                format!(
                    "Error loading targets in package `{}` for target `{}`",
                    target.pkg(),
                    target
                )
            })?
            .resolve_target(target.name())?
            .dupe())
    }
}
