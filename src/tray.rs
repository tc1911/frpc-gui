use crate::TrayCmd;
use image::GenericImageView;
use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;
use std::sync::LazyLock;
use std::sync::mpsc::Sender;

static ICON: LazyLock<ksni::Icon> = LazyLock::new(|| {
    let img = image::load_from_memory_with_format(
        include_bytes!("../assets/icon.png"),
        image::ImageFormat::Png,
    )
    .expect("内嵌图标应当有效");
    let (w, h) = img.dimensions();
    let mut data = img.into_rgba8().into_vec();
    for px in data.chunks_exact_mut(4) {
        px.rotate_right(1); // rgba -> argb
    }
    ksni::Icon {
        width: w as i32,
        height: h as i32,
        data,
    }
});

pub struct TrayHandle {
    _handle: ksni::blocking::Handle<Tray>,
}

pub struct Tray {
    tx: Sender<TrayCmd>,
    running: usize,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "frpc-gui".into()
    }

    fn title(&self) -> String {
        if self.running == 0 {
            "frpc GUI".into()
        } else {
            format!("frpc GUI · {} 条在跑", self.running)
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ICON.clone()]
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: "显示窗口".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCmd::Show);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "全部启动".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCmd::StartAll);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "全部停止".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCmd::StopAll);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "退出".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.tx.send(TrayCmd::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn spawn(tx: Sender<TrayCmd>) -> Option<TrayHandle> {
    let tray = Tray { tx, running: 0 };
    match tray.spawn() {
        Ok(handle) => Some(TrayHandle { _handle: handle }),
        Err(e) => {
            eprintln!("托盘启动失败: {e}");
            None
        }
    }
}
