from pathlib import Path
p=Path('src/md.rs');s=p.read_text(encoding='utf8');s+='''
/// Reine Standard-Changelog-Ausgabe ohne Titel- oder Datumsheader.
pub fn changelog_filtered(update: &Update, only_done: bool) -> String {
    let mut out=String::new();
    for kind in ChangeKind::ALL {
        let tasks: Vec<_>=update.tasks.iter().filter(|t|t.kind==kind && (!only_done || t.done)).collect();
        if tasks.is_empty(){continue;}
        out.push_str(&format!("### {}\\n",kind.label()));
        for t in tasks {out.push_str(&format!("* {}: {}\\n",t.title.trim().replace('\\n'," "),t.description.trim().replace('\\n'," ")));}
        out.push('\\n');
    }
    out.trim_end().to_owned()
}
''';p.write_text(s,encoding='utf8')
p=Path('src/ui/detail.rs');s=p.read_text(encoding='utf8').replace('md::changelog(u,self.changelog_done_only)','md::changelog_filtered(u,self.changelog_done_only)').replace('use super::list::{progress_bar, update_menu};','use super::list::update_menu;');p.write_text(s,encoding='utf8')
