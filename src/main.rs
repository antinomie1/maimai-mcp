pub mod config;
pub mod core;
pub mod draw;
pub mod error;
pub mod tools;

use env_logger::Target;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    tool, tool_router, ServiceExt,
};
use tools::{
    b50::{B50Tool, MaiB50Params},
    fortune::{FortuneTool, MaiFortuneParams},
    group_rank::{GroupRankTool, MaiGroupRankParams},
    music::{MaiAliasQueryParams, MaiChartInfoParams, MaiSearchSongParams, MusicTools},
    progress::{MaiLevelProgressParams, MaiPlateTableParams, ProgressTools},
    random::{MaiRandomSongParams, MaiWhatParams, RandomTools},
    score::{MaiPlayScoreParams, MaiRiseScoreParams, ScoreTools},
    user::{MaiBindPlayerParams, MaiUserSettingsParams, UserTools},
};

#[derive(Clone, Default)]
pub struct MaimaiMcpServer;

#[tool_router(server_handler)]
impl MaimaiMcpServer {
    pub fn new() -> Self {
        Self
    }

    #[tool(description = "Ping server to verify maimai-mcp connection")]
    async fn ping(&self) -> Result<CallToolResult, String> {
        Ok(CallToolResult::success(vec![ContentBlock::text("pong")]))
    }

    #[tool(description = "Query player's Best 50 (B35 + B15) scores, Rating, and generate B50 overview image card")]
    async fn mai_b50(&self, Parameters(params): Parameters<MaiB50Params>) -> Result<CallToolResult, String> {
        B50Tool::execute(params).await
    }

    #[tool(description = "Query player's play score, achievements, DX rating and rank for a specific song across all difficulties and generate score结算 image card")]
    async fn mai_play_score(&self, Parameters(params): Parameters<MaiPlayScoreParams>) -> Result<CallToolResult, String> {
        ScoreTools::play_score(params).await
    }

    #[tool(description = "Query song chart details, difficulty levels, internal constants (ds), charter, note counts and generate chart preview image card")]
    async fn mai_chart_info(&self, Parameters(params): Parameters<MaiChartInfoParams>) -> Result<CallToolResult, String> {
        MusicTools::chart_info(params).await
    }

    #[tool(description = "Calculate score rise recommendations to step up DX Rating based on B50 floor and generate rise recommendation image")]
    async fn mai_rise_score(&self, Parameters(params): Parameters<MaiRiseScoreParams>) -> Result<CallToolResult, String> {
        ScoreTools::rise_score(params).await
    }

    #[tool(description = "Search songs by title, artist, genre, version, or alias")]
    async fn mai_search_song(&self, Parameters(params): Parameters<MaiSearchSongParams>) -> Result<CallToolResult, String> {
        MusicTools::search_song(params).await
    }

    #[tool(description = "Query all known aliases and nicknames for a song or reverse search song by alias")]
    async fn mai_alias_query(&self, Parameters(params): Parameters<MaiAliasQueryParams>) -> Result<CallToolResult, String> {
        MusicTools::alias_query(params).await
    }

    #[tool(description = "Pick a random song filtered by difficulty level, constant, or type (SD/DX) and generate song image card")]
    async fn mai_random_song(&self, Parameters(params): Parameters<MaiRandomSongParams>) -> Result<CallToolResult, String> {
        RandomTools::random_song(params).await
    }

    #[tool(description = "Recommend what to play / eat today with randomized suggestions and generate song image card")]
    async fn mai_what(&self, Parameters(params): Parameters<MaiWhatParams>) -> Result<CallToolResult, String> {
        RandomTools::mai_what(params).await
    }

    #[tool(description = "Query player's rating level table completion (e.g. 13+, 14) and generate level progress grid image")]
    async fn mai_level_progress(&self, Parameters(params): Parameters<MaiLevelProgressParams>) -> Result<CallToolResult, String> {
        ProgressTools::level_progress(params).await
    }

    #[tool(description = "Query player's plate completion progress (舞, 霸者, 将, 极, 神, 舞舞) and generate plate table progress image")]
    async fn mai_plate_table(&self, Parameters(params): Parameters<MaiPlateTableParams>) -> Result<CallToolResult, String> {
        ProgressTools::plate_table(params).await
    }

    #[tool(description = "Bind user QQ to DivingFish username, LXNS token, default score source or theme")]
    async fn mai_bind_player(&self, Parameters(params): Parameters<MaiBindPlayerParams>) -> Result<CallToolResult, String> {
        UserTools::bind_player(params).await
    }

    #[tool(description = "View user settings, bound accounts, default score source and theme")]
    async fn mai_user_settings(&self, Parameters(params): Parameters<MaiUserSettingsParams>) -> Result<CallToolResult, String> {
        UserTools::user_settings(params).await
    }

    #[tool(description = "Draw today's maimai fortune (运势值, 宜/忌, 幸运曲目) and generate daily fortune image card")]
    async fn mai_fortune(&self, Parameters(params): Parameters<MaiFortuneParams>) -> Result<CallToolResult, String> {
        FortuneTool::execute(params).await
    }

    #[tool(description = "Generate group leaderboard ranking of member DX ratings")]
    async fn mai_group_rank(&self, Parameters(params): Parameters<MaiGroupRankParams>) -> Result<CallToolResult, String> {
        GroupRankTool::execute(params).await
    }
}



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(Target::Stderr)
        .init();

    log::info!("Starting maimai-mcp server (Rust)...");

    let server = MaimaiMcpServer::new();
    let service = server.serve((tokio::io::stdin(), tokio::io::stdout())).await?;
    service.waiting().await?;

    Ok(())
}
