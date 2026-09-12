use clap::{Parser, Subcommand};

mod commands;
mod utils;

use commands::{
    atlas::{run_atlas, AtlasSubcommand},
    bench::{run_bench, BenchSubcommand},
    ctm::{run_ctm_test, CtmTestArgs},
    export::{run_export, ExportSubcommand},
    model::{run_model, ModelSubcommand},
    verify::{run_verify_cull, VerifyCullArgs},
};

#[derive(Parser, Debug)]
#[command(
    name = "mtk",
    author = "Mozi <mozi1924@arasaka.ltd>",
    version = "0.1.0",
    about = "Unified CLI toolkit for Minecraft 3D model baking, texture atlas generation, and mesh optimization",
    long_about = "MoziToolKit (mtk) - A high-performance pipeline for Minecraft voxel meshing, universal model baking, CTM / OptiFine pack parsing, isolated multi-category atlas generation, and culling rule verification."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Texture atlas baking and sprite address map generation
    #[command(subcommand)]
    Atlas(AtlasSubcommand),

    /// Model baking inspection and batch data serialization
    #[command(subcommand)]
    Model(ModelSubcommand),

    /// Export baked models and textures to Wavefront OBJ / MTL
    #[command(subcommand)]
    Export(ExportSubcommand),

    /// Continuity and OptiFine Connected Textures (CTM) rule parsing and atlas testing
    Ctm {
        #[command(flatten)]
        args: CtmTestArgs,
    },

    /// Verification utilities (e.g. face culling rules against reference test cases)
    Verify {
        #[command(subcommand)]
        sub: VerifyCommands,
    },

    /// High-performance benchmarks for model baking and parallel throughput
    #[command(subcommand)]
    Bench(BenchSubcommand),
}

#[derive(Subcommand, Debug)]
enum VerifyCommands {
    /// Verify face culling logic against JSON test cases
    Cull(VerifyCullArgs),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Atlas(sub) => run_atlas(sub),
        Commands::Model(sub) => run_model(sub),
        Commands::Export(sub) => run_export(sub),
        Commands::Ctm { args } => run_ctm_test(args),
        Commands::Verify { sub } => match sub {
            VerifyCommands::Cull(args) => run_verify_cull(args),
        },
        Commands::Bench(sub) => run_bench(sub),
    }
}
