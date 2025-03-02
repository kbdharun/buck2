/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use async_trait::async_trait;
use buck2_cli_proto::ConfiguredTargetsRequest;
use buck2_cli_proto::ConfiguredTargetsResponse;
use buck2_client_ctx::client_ctx::ClientCommandContext;
use buck2_client_ctx::common::CommonBuildConfigurationOptions;
use buck2_client_ctx::common::CommonCommandOptions;
use buck2_client_ctx::common::CommonConsoleOptions;
use buck2_client_ctx::common::CommonDaemonCommandOptions;
use buck2_client_ctx::daemon::client::BuckdClientConnector;
use buck2_client_ctx::daemon::client::NoPartialResultHandler;
use buck2_client_ctx::exit_result::ExitResult;
use buck2_client_ctx::streaming::StreamingCommand;
use clap::ArgMatches;
use gazebo::prelude::SliceExt;

/// Resolve target patterns to configured targets.
#[derive(Debug, clap::Parser)]
#[clap(name = "ctargets")]
pub struct ConfiguredTargetsCommand {
    #[clap(flatten)]
    common_opts: CommonCommandOptions,

    /// Skip missing targets from `BUCK` files when non-glob pattern is specified.
    /// This option does not skip missing packages
    /// and does not ignore errors of `BUCK` file evaluation.
    #[clap(long)]
    skip_missing_targets: bool,

    /// Patterns to interpret.
    #[clap(name = "TARGET_PATTERNS")]
    patterns: Vec<String>,
}

#[async_trait]
impl StreamingCommand for ConfiguredTargetsCommand {
    const COMMAND_NAME: &'static str = "ctargets";

    async fn exec_impl(
        self,
        buckd: &mut BuckdClientConnector,
        matches: &ArgMatches,
        ctx: &mut ClientCommandContext<'_>,
    ) -> ExitResult {
        let context = Some(ctx.client_context(
            &self.common_opts.config_opts,
            matches,
            self.sanitized_argv(),
        )?);
        let ConfiguredTargetsResponse {
            serialized_targets_output,
        } = buckd
            .with_flushing()
            .ctargets(
                ConfiguredTargetsRequest {
                    context,
                    target_patterns: self.patterns.map(|pat| buck2_data::TargetPattern {
                        value: pat.to_owned(),
                    }),
                    skip_missing_targets: self.skip_missing_targets,
                },
                ctx.stdin()
                    .console_interaction_stream(&self.common_opts.console_opts),
                &mut NoPartialResultHandler,
            )
            .await??;

        buck2_client_ctx::print!("{}", serialized_targets_output)?;

        ExitResult::success()
    }

    fn console_opts(&self) -> &CommonConsoleOptions {
        &self.common_opts.console_opts
    }

    fn event_log_opts(&self) -> &CommonDaemonCommandOptions {
        &self.common_opts.event_log_opts
    }

    fn common_opts(&self) -> &CommonBuildConfigurationOptions {
        &self.common_opts.config_opts
    }
}
