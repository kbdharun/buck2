/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use async_trait::async_trait;
use buck2_cli_proto::CqueryRequest;
use buck2_client_ctx::client_ctx::ClientCommandContext;
use buck2_client_ctx::common::CommonBuildConfigurationOptions;
use buck2_client_ctx::common::CommonCommandOptions;
use buck2_client_ctx::common::CommonConsoleOptions;
use buck2_client_ctx::common::CommonDaemonCommandOptions;
use buck2_client_ctx::daemon::client::BuckdClientConnector;
use buck2_client_ctx::daemon::client::StdoutPartialResultHandler;
use buck2_client_ctx::exit_result::ExitResult;
use buck2_client_ctx::streaming::StreamingCommand;

use crate::commands::query::common::CommonQueryOptions;

/// Perform queries on the configured target graph.
///
/// The configured target graph includes information about the configuration (platforms) and
/// transitions involved in building targets. In the configured graph, `selects` are fully
/// resolved. The same target may appear in multiple different configurations (when printed,
/// the configuration is after the target in parentheses).
///
/// A user can specify a `--target-universe` flag to control how literals are resolved. When
/// provided, any literals will resolve to all matching targets within the universe (which
/// includes the targets passed as the universe and all transitive deps of them).
/// When not provided, we implicitly set the universe to be rooted at every target literal
/// in the `cquery`.
///
/// Run `buck2 docs cquery` for more documentation about the functions available in cquery
/// expressions.
///
/// Examples:
///
/// Print all the attributes of a target
///
/// `buck2 cquery //java/com/example/app:amazing --output-all-attributes`
///
/// List the deps of a target (special characters in a target will require quotes):
///
/// `buck2 cquery 'deps("//java/com/example/app:amazing+more")'`
#[derive(Debug, clap::Parser)]
#[clap(name = "cquery")]
pub struct CqueryCommand {
    #[clap(flatten)]
    common_opts: CommonCommandOptions,

    #[clap(flatten)]
    query_common: CommonQueryOptions,

    #[clap(
        long,
        use_delimiter = true,
        help = "Comma separated list of targets at which to root the queryable universe.
                This is useful since targets can exist in multiple configurations. While
                this argument isn't required, it's recommended for most non-trivial queries.
                In the absence of this argument, buck2 will use the target literals
                in your cquery expression as the argument to this."
    )]
    target_universe: Vec<String>,

    #[clap(
        long,
        help = "Show the providers of the query result instead of the attributes and labels"
    )]
    show_providers: bool,

    #[allow(rustdoc::bare_urls)]
    /// Enable deprecated `owner()` function behavior.
    ///
    /// See this post https://fburl.com/1mf2d2xj for details.
    #[clap(long)]
    deprecated_owner: bool,

    #[allow(rustdoc::bare_urls)]
    /// Enable correct `owner()` function behavior.
    ///
    /// See this post https://fburl.com/1mf2d2xj for details.
    #[clap(long)]
    correct_owner: bool,
}

#[async_trait]
impl StreamingCommand for CqueryCommand {
    const COMMAND_NAME: &'static str = "cquery";

    async fn exec_impl(
        self,
        buckd: &mut BuckdClientConnector,
        matches: &clap::ArgMatches,
        ctx: &mut ClientCommandContext<'_>,
    ) -> ExitResult {
        let (query, query_args) = self.query_common.get_query();
        let unstable_output_format = self.query_common.output_format() as i32;
        let output_attributes = self.query_common.attributes.get()?;
        let context = ctx.client_context(
            &self.common_opts.config_opts,
            matches,
            self.sanitized_argv(),
        )?;

        let correct_owner = match (self.correct_owner, self.deprecated_owner) {
            (true, false) => true,
            (false, true) => false,
            (false, false) => true,
            (true, true) => {
                return ExitResult::bail(
                    "Cannot specify both --correct-owner and --deprecated-owner",
                );
            }
        };

        let response = buckd
            .with_flushing()
            .cquery(
                CqueryRequest {
                    query,
                    query_args,
                    context: Some(context),
                    output_attributes,
                    target_universe: self.target_universe,
                    show_providers: self.show_providers,
                    unstable_output_format,
                    correct_owner,
                },
                ctx.stdin()
                    .console_interaction_stream(&self.common_opts.console_opts),
                &mut StdoutPartialResultHandler,
            )
            .await??;

        for message in &response.error_messages {
            buck2_client_ctx::eprintln!("{}", message)?;
        }

        if !response.error_messages.is_empty() {
            ExitResult::failure()
        } else {
            ExitResult::success()
        }
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
