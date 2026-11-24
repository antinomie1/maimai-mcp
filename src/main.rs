use env_logger::Target;
use maimai_mcp::core::update;
use maimai_mcp::tools::{b50, finish, music, progress, score, user};
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};

#[derive(Clone, Default)]
pub struct MaimaiMcpServer;

macro_rules! run {
    ($f:expr, $fmt:expr) => {
        Ok(finish($f.await, $fmt))
    };
}

#[tool_router(server_handler)]
impl MaimaiMcpServer {
    #[tool(description = "B50 / AP50 image (b50, ap50). Query by QQ (bound source) or DivingFish username.")]
    async fn mai_b50(&self, Parameters(p): Parameters<b50::B50Params>) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(b50::b50(p), fmt.as_deref())
    }

    #[tool(description = "User's records on all difficulties of one song (info / minfo)")]
    async fn mai_play_score(
        &self,
        Parameters(p): Parameters<score::PlayScoreParams>,
    ) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(score::play_score(p), fmt.as_deref())
    }

    #[tool(
        description = "Global play statistics of a chart: play count, fitted difficulty, average achievement (ginfo)"
    )]
    async fn mai_song_stats(
        &self,
        Parameters(p): Parameters<score::SongStatsParams>,
    ) -> Result<CallToolResult, String> {
        run!(score::song_stats(p), None)
    }

    #[tool(
        description = "Allowed TAP GREAT count for a target achievement line (分数线). Call without arguments for help."
    )]
    async fn mai_score_line(
        &self,
        Parameters(p): Parameters<score::ScoreLineParams>,
    ) -> Result<CallToolResult, String> {
        run!(score::score_line(p), None)
    }

    #[tool(description = "Chart recommendations that raise the user's DX Rating (我要上分 / 我要在13+上5分)")]
    async fn mai_rise_score(&self, Parameters(p): Parameters<score::RiseParams>) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(score::rise_score(p), fmt.as_deref())
    }

    #[tool(description = "Chart info card of a song found by alias, id or title (XXX是什么歌 / id xxx)")]
    async fn mai_chart_info(
        &self,
        Parameters(p): Parameters<music::ChartInfoParams>,
    ) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(music::chart_info(p), fmt.as_deref())
    }

    #[tool(
        description = "Search songs by title, internal constant (定数), bpm, artist (曲师) or charter (谱师) (查歌)"
    )]
    async fn mai_search_song(&self, Parameters(p): Parameters<music::SearchParams>) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(music::search_song(p), fmt.as_deref())
    }

    #[tool(description = "All known aliases of a song (XXX有什么别名)")]
    async fn mai_alias_query(&self, Parameters(p): Parameters<music::AliasParams>) -> Result<CallToolResult, String> {
        run!(music::alias_query(p), None)
    }

    #[tool(description = "Random song of a level, optional difficulty colour and SD/DX type (随个[dx][紫]13+)")]
    async fn mai_random_song(&self, Parameters(p): Parameters<music::RandomParams>) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(music::random_song(p), fmt.as_deref())
    }

    #[tool(
        description = "What to play today (mai什么); rise=true picks a chart that can raise rating (今日mai打什么上分)"
    )]
    async fn mai_what(&self, Parameters(p): Parameters<music::WhatParams>) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(music::mai_what(p), fmt.as_deref())
    }

    #[tool(description = "Today's maimai fortune: luck value, do/don't list and a recommended song (今日运势)")]
    async fn mai_fortune(&self, Parameters(p): Parameters<music::FortuneParams>) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(music::fortune(p), fmt.as_deref())
    }

    #[tool(description = "Level constant table image for lv7-15 without user data (13+定数表)")]
    async fn mai_rating_table(
        &self,
        Parameters(p): Parameters<progress::RatingTableParams>,
    ) -> Result<CallToolResult, String> {
        run!(progress::rating_table(p), None)
    }

    #[tool(description = "User's completion table of a level, ranks or fc/ap/fs/fdx goal (13+完成表 / 14ap完成表)")]
    async fn mai_level_table(
        &self,
        Parameters(p): Parameters<progress::LevelTableParams>,
    ) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(progress::level_table(p), fmt.as_deref())
    }

    #[tool(
        description = "Progress of a level towards a goal with completed / unfinished / unplayed lists (14 sss 进度 / 14ap未完成进度)"
    )]
    async fn mai_level_progress(
        &self,
        Parameters(p): Parameters<progress::LevelProgressParams>,
    ) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(progress::level_progress(p), fmt.as_deref())
    }

    #[tool(description = "User's scores of a level or constant sorted by achievement (13+分数列表 / 13.7分数列表)")]
    async fn mai_level_score_list(
        &self,
        Parameters(p): Parameters<progress::ScoreListParams>,
    ) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(progress::level_score_list(p), fmt.as_deref())
    }

    #[tool(
        description = "Plate completion table (祝极完成表 / 霸者完成表 2); without version returns plate requirements (牌子条件)"
    )]
    async fn mai_plate_table(
        &self,
        Parameters(p): Parameters<progress::PlateParams>,
    ) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(progress::plate_table(p), fmt.as_deref())
    }

    #[tool(description = "Unfinished charts of a plate per difficulty (舞将进度 / 彩神进度)")]
    async fn mai_plate_progress(
        &self,
        Parameters(p): Parameters<progress::PlateParams>,
    ) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(progress::plate_progress(p), fmt.as_deref())
    }

    #[tool(
        description = "DivingFish OAuth binding (绑定水鱼): call without code for the link, then with the confirmation code"
    )]
    async fn mai_bind_divingfish(
        &self,
        Parameters(p): Parameters<user::BindDfParams>,
    ) -> Result<CallToolResult, String> {
        run!(user::bind_divingfish(p), None)
    }

    #[tool(
        description = "LXNS OAuth binding (lxbind): call without code for the link, then with the authorization code"
    )]
    async fn mai_bind_lxns(&self, Parameters(p): Parameters<user::BindLxParams>) -> Result<CallToolResult, String> {
        run!(user::bind_lxns(p), None)
    }

    #[tool(description = "View settings, switch score source (数据源) or theme (主题)")]
    async fn mai_user_settings(
        &self,
        Parameters(p): Parameters<user::SettingsParams>,
    ) -> Result<CallToolResult, String> {
        run!(user::user_settings(p), None)
    }

    #[tool(
        description = "DivingFish prober rating ranking, find a username, or the QQ user's own rank (查看排名 / 我的排名)"
    )]
    async fn mai_rating_ranking(
        &self,
        Parameters(p): Parameters<b50::RankingParams>,
    ) -> Result<CallToolResult, String> {
        let fmt = p.output_format.clone();
        run!(b50::rating_ranking(p), fmt.as_deref())
    }

    #[tool(description = "Leaderboard of group members by DX Rating")]
    async fn mai_group_rank(&self, Parameters(p): Parameters<b50::GroupRankParams>) -> Result<CallToolResult, String> {
        run!(b50::group_rank(p), None)
    }

    #[tool(
        description = "Refresh song / alias / plate data online and re-render tables (更新maimai数据 / 更新别名库 / 更新定数表)"
    )]
    async fn mai_update_data(&self, Parameters(p): Parameters<user::UpdateParams>) -> Result<CallToolResult, String> {
        run!(user::update_data(p), None)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).target(Target::Stderr).init();
    log::info!("Starting maimai-mcp server");
    update::spawn_scheduler();
    let service = MaimaiMcpServer.serve((tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;
    Ok(())
}
