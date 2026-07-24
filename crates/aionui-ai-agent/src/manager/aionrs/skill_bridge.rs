use std::io;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tracing::info;

/// Stage Zigo's branded workspace skills for the upstream AionRS loader.
///
/// AionRS 0.2.3 only scans `<root>/.aionrs/skills`. Zigo keeps the canonical
/// workspace contract at `.zigo/skills`, so the compatibility tree lives
/// outside the user workspace and is passed through `extra_skill_dirs`.
pub async fn stage_zigo_skills(
    workspace: &Path,
    session_directory: &Path,
    conversation_id: &str,
) -> io::Result<Option<PathBuf>> {
    let source = workspace.join(".zigo").join("skills");
    if !source.is_dir() {
        return Ok(None);
    }

    let Some(data_dir) = session_directory.parent() else {
        return Ok(None);
    };
    let stage_key = hex::encode(Sha256::digest(conversation_id.as_bytes()));
    let stage_root = data_dir.join("aionrs-skill-bridges").join(stage_key);
    let stage_skills = stage_root.join(".aionrs").join("skills");

    match tokio::fs::remove_dir_all(&stage_root).await {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    tokio::fs::create_dir_all(&stage_skills).await?;
    copy_dir_recursive(&source, &stage_skills).await?;

    info!(
        conversation_id,
        source = %source.display(),
        stage = %stage_skills.display(),
        "Staged .zigo skills for AionRS discovery"
    );
    Ok(Some(stage_root))
}

fn copy_dir_recursive<'a>(
    source: &'a Path,
    destination: &'a Path,
) -> std::pin::Pin<Box<dyn Future<Output = io::Result<()>> + Send + 'a>> {
    Box::pin(async move {
        tokio::fs::create_dir_all(destination).await?;
        let mut entries = tokio::fs::read_dir(source).await?;
        while let Some(entry) = entries.next_entry().await? {
            let source_path = entry.path();
            let destination_path = destination.join(entry.file_name());
            let metadata = tokio::fs::metadata(&source_path).await?;
            if metadata.is_dir() {
                copy_dir_recursive(&source_path, &destination_path).await?;
            } else if metadata.is_file() {
                tokio::fs::copy(&source_path, &destination_path).await?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stages_branded_skills_in_upstream_discovery_shape() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let sessions = temp.path().join("data").join("aionrs-sessions");
        let skill = workspace.join(".zigo/skills/zigo-config");
        tokio::fs::create_dir_all(skill.join("references")).await.unwrap();
        let manifest = "---\nname: zigo-config\ndescription: Configure Zigo\n---\nzigo config";
        tokio::fs::write(skill.join("SKILL.md"), manifest).await.unwrap();
        tokio::fs::write(skill.join("references/guide.md"), "guide")
            .await
            .unwrap();

        let root = stage_zigo_skills(&workspace, &sessions, "conv-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            tokio::fs::read_to_string(root.join(".aionrs/skills/zigo-config/SKILL.md"))
                .await
                .unwrap(),
            manifest
        );
        assert_eq!(
            tokio::fs::read_to_string(root.join(".aionrs/skills/zigo-config/references/guide.md"))
                .await
                .unwrap(),
            "guide"
        );
        let loaded = aion_agent::skills::loader::load_all_skills(&workspace, &[root], false, None).await;
        assert!(
            loaded.iter().any(|skill| skill.name == "zigo-config"),
            "the upstream AionRS loader must discover the branded Zigo skill"
        );
        assert!(!workspace.join(".aionrs").exists());
    }
}
