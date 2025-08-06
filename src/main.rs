use clap::{Parser, Subcommand};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path; // 移除未使用的 PathBuf 导入
use zip::ZipArchive;
use anyhow::{Result, Context};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration; // 导入 Duration 类型以正确设置进度条
// 主入口文件 main.rs
#[derive(Parser)]
#[command(name = "奥科戈包管理器", about = "简单的中文包管理工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 获取并安装指定包（下载 + 解压）
    Get {
        /// 包名
        package: String,

        /// ZIP 文件名（可选），例如 mypkg.zip
        #[arg(short, long)]
        file: Option<String>,
    },

    /// 删除指定包（卸载）
    Remove {
        /// 包名
        package: String,
    },

    /// 解压已下载的 zip 包
    Extract {
        /// 包名
        package: String,
    },
}

const BASE_URL: &str = "https://aokege.github.io/zhucechu";
const BASE_DIR: &str = "./packages";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Get { package, file } => {
            install_package(package, file.as_deref()).await?
        }
        Commands::Remove { package } => uninstall_package(package)?,
        Commands::Extract { package } => unzip_package(package)?,
    }

    Ok(())
}

async fn install_package(package: &str, filename: Option<&str>) -> Result<()> {
    
    let default_zip_name = format!("{package}.zip");
    let zip_name = filename.unwrap_or(&default_zip_name);
    let url = format!("{BASE_URL}/zujian/{package}/{zip_name}");

    let output_dir = Path::new(BASE_DIR).join(package);
    let zip_path = Path::new(BASE_DIR).join(zip_name);

    println!("⬇️ 正在下载: {}", url);
    std::fs::create_dir_all(BASE_DIR)?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    // 修复：`enable_steady_tick` 需要一个 `Duration` 类型
    pb.enable_steady_tick(Duration::from_millis(120)); 
    pb.set_message("下载中...");

    let res = reqwest::get(&url).await.context("网络请求失败")?;
    if !res.status().is_success() {
        pb.finish_and_clear();
        anyhow::bail!("请求失败，状态码: {}", res.status());
    }
    let bytes = res.bytes().await.context("读取内容失败")?;

    let mut file = File::create(&zip_path)?;
    file.write_all(&bytes)?;
    pb.finish_with_message("✅ 下载完成");

    println!("📦 正在解压...");
    unzip_from_path(&zip_path, &output_dir)?;
    println!("✅ 安装完成: {:?}", output_dir);

    Ok(())
}

fn uninstall_package(package: &str) -> Result<()> {
    let dir = Path::new(BASE_DIR).join(package);

    if dir.exists() && dir.is_dir() {
        fs::remove_dir_all(&dir).context("删除包文件夹失败")?;
        println!("🗑️ 成功卸载包: {}", package);
    } else {
        println!("❌ 找不到包: {}", package);
    }

    Ok(())
}

fn unzip_package(package: &str) -> Result<()> {
    let zip_path = Path::new(BASE_DIR).join(format!("{package}.zip"));
    let output_dir = Path::new(BASE_DIR).join(package);

    if !zip_path.exists() {
        anyhow::bail!("未找到已下载的压缩包: {:?}", zip_path);
    }

    println!("📦 正在解压: {:?}", zip_path);
    unzip_from_path(&zip_path, &output_dir)?;
    println!("✅ 解压完成: {:?}", output_dir);

    Ok(())
}

fn unzip_from_path(zip_path: &Path, output_dir: &Path) -> Result<()> {
    let file = File::open(zip_path).context("打开压缩包失败")?;
    let mut archive = ZipArchive::new(file).context("读取压缩包失败")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = output_dir.join(file.name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}
