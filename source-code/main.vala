using Gtk;
using GLib;
using Gdk;

namespace HackerMode {
    private struct SettingAction {
        public string label;
        public string cmd;
    }

    const string CSS = """
    * {
    font-family: 'Courier New', Monospace;
    transition: all 0.3s ease;
}
.bg-gradient {
background: linear-gradient(135deg, #0d0d0d, #1a1a1a);
}
.neon-text {
font-size: 72px;
font-weight: bold;
color: #0ff;
text-shadow: 0 0 10px #0ff, 0 0 20px #0ff, 0 0 40px #0ff;
animation: neon 1.5s ease-in-out infinite alternate;
}
.launcher-text {
font-size: 42px;
font-weight: bold;
color: #0ff;
text-shadow: 0 0 10px #0ff, 0 0 25px #0ff;
}
@keyframes neon {
from { text-shadow: 0 0 10px #0ff, 0 0 20px #0ff, 0 0 30px #0ff; }
to { text-shadow: 0 0 20px #0ff, 0 0 30px #0ff, 0 0 50px #0ff; }
}
.launcher-btn {
background: rgba(30,30,30,0.8);
border-radius: 24px;
padding: 30px;
box-shadow: 0 8px 30px rgba(0,255,255,0.4);
}
.launcher-btn:hover {
transform: scale(1.1);
box-shadow: 0 20px 50px rgba(0,255,255,0.7);
}
.bottom-btn {
background: rgba(40,40,40,0.95);
border-radius: 14px;
padding: 14px 28px;
font-weight: bold;
font-size: 18px;
}
.bottom-btn:hover {
background: rgba(70,70,70,0.95);
transform: translateY(-4px);
}
.setting-panel {
background: rgba(40,40,40,0.75);
border-radius: 18px;
padding: 25px;
box-shadow: 0 10px 40px rgba(0,255,255,0.25);
}
.setting-btn {
background: rgba(60,60,60,0.85);
margin: 8px 0;
border-radius: 12px;
font-size: 16px;
}
.setting-btn:hover {
background: rgba(90,90,90,0.95);
transform: translateY(-3px);
}
""";

    public class Application : Gtk.Application {
        public Application() {
            Object(application_id: "com.voidarc.hackermode", flags: ApplicationFlags.DEFAULT_FLAGS);
        }

        protected override void activate() {
            var win = new MainWindow(this);
            win.present();
        }
    }

    public class MainWindow : ApplicationWindow {
        private Stack stack;
        private PopoverMenu hacker_menu;
        private HashTable<string, int64?> last_launch = new HashTable<string, int64?>(str_hash, str_equal);

        public MainWindow(Application app) {
            Object(application: app);
            this.title = "Hacker Mode";
            this.fullscreen();

            var css_provider = new CssProvider();
            css_provider.load_from_string(CSS);
            StyleContext.add_provider_for_display(Gdk.Display.get_default(), css_provider, STYLE_PROVIDER_PRIORITY_APPLICATION);

            // CTRL+SHIFT+M (działa niezależnie od CapsLock/Shift)
            var key_controller = new EventControllerKey();
            key_controller.key_pressed.connect((keyval, keycode, state) => {
                uint lower = Gdk.keyval_to_lower(keyval);
                if ((state & ModifierType.CONTROL_MASK) != 0 && (state & ModifierType.SHIFT_MASK) != 0 && lower == Gdk.Key.m) {
                    hacker_menu.popup();
                    return true;
                }
                return false;
            });
            ((Gtk.Widget)this).add_controller(key_controller);

            var overlay = new Overlay();
            this.set_child(overlay);

            stack = new Stack();
            stack.transition_type = StackTransitionType.SLIDE_LEFT_RIGHT;
            overlay.set_child(stack);

            build_main_page();
            build_settings_page();
            build_hacker_menu();
        }

        private void build_main_page() {
            var box = new Box(Orientation.VERTICAL, 0);
            box.add_css_class("bg-gradient");

            var title = new Label("Hacker Mode");
            title.add_css_class("neon-text");
            title.margin_top = 60;
            title.margin_bottom = 100;
            box.append(title);

            var grid = new Grid();
            grid.column_spacing = 100;
            grid.row_spacing = 100;
            grid.halign = grid.valign = Align.CENTER;

            var launchers = new LauncherInfo[] {
                LauncherInfo() { name = "Steam", command = "flatpak run com.valvesoftware.Steam -gamepadui", internet = true },
                LauncherInfo() { name = "Heroic", command = "flatpak run com.heroicgameslauncher.hgl", internet = true },
                LauncherInfo() { name = "HyperPlay", command = "flatpak run xyz.hyperplay.HyperPlay", internet = true },
                LauncherInfo() { name = "Lutris", command = "lutris", internet = false }
            };

            int col = 0, row = 0;
            foreach (var l in launchers) {
                var btn = create_launcher_button(l);
                btn.clicked.connect(() => { launch_application.begin(l); });
                grid.attach(btn, col++, row, 1, 1);
                if (col == 4) { col = 0; row++; }
            }

            box.append(grid);

            var bottom_box = new Box(Orientation.HORIZONTAL, 40);
            bottom_box.margin_start = bottom_box.margin_bottom = 50;

            var settings_btn = new Button.with_label("Settings");
            settings_btn.add_css_class("bottom-btn");
            settings_btn.clicked.connect(() => { stack.visible_child_name = "settings"; });

            var hacker_btn = new Button.with_label("Hacker Menu");
            hacker_btn.add_css_class("bottom-btn");
            hacker_btn.clicked.connect(() => { hacker_menu.popup(); });

            bottom_box.append(settings_btn);
            bottom_box.append(hacker_btn);

            box.append(bottom_box);

            stack.add_named(box, "main");
        }

        private Button create_launcher_button(LauncherInfo info) {
            var label = new Label(info.name);
            label.add_css_class("launcher-text");

            var btn = new Button();
            btn.child = label;
            btn.add_css_class("launcher-btn");
            btn.width_request = 260;
            btn.height_request = 260;

            return btn;
        }

        private async void launch_application(LauncherInfo info) {
            int64 now = GLib.get_real_time() / 1000000;
            int64? last = last_launch.get(info.name);
            if (last != null && now - last < 60) {
                message(@"Cooldown: poczekaj jeszcze $(60 - (int)(now - last)) sekund");
                return;
            }

            if (info.internet && !has_internet()) {
                message("Brak internetu – wymagany dla " + info.name);
                return;
            }

            last_launch.set(info.name, now);
            this.set_visible(false);

            try {
                string[] argv;
                Shell.parse_argv(info.command, out argv);
                var subprocess = new Subprocess.newv(argv, SubprocessFlags.NONE);
                yield subprocess.wait_async();
                Idle.add(() => {
                    this.present();
                    this.fullscreen();
                    return false;
                });
            } catch (Error e) {
                message("Nie można uruchomić: " + info.name);
                this.present();
            }

            Timeout.add(4000, () => {
                try {
                    Process.spawn_command_line_async(@"xdotool search --name \"$(info.name)\" key F11");
                } catch {}
                return Source.REMOVE;
            });
        }

        private bool has_internet() {
            string output = "";
            try {
                Process.spawn_command_line_sync("nmcli networking connectivity",
                                                out output,
                                                null,
                                                null);
                return output.strip() == "full";
            } catch {
                return false;
            }
        }

        private void build_settings_page() {
            var scrolled = new ScrolledWindow();
            var box = new Box(Orientation.VERTICAL, 40);
            box.margin_top = box.margin_bottom = 50;
            scrolled.set_child(box);

            var grid = new Grid();
            grid.column_spacing = 60;
            grid.row_spacing = 60;
            grid.margin_start = grid.margin_end = 100;

            var audio_panel = create_setting_panel("Dźwięk", {
                SettingAction() { label = "Zwiększ głośność", cmd = "pactl set-sink-volume @DEFAULT_SINK@ +5%" },
                SettingAction() { label = "Zmniejsz głośność", cmd = "pactl set-sink-volume @DEFAULT_SINK@ -5%" },
                SettingAction() { label = "Wycisz/Włącz", cmd = "pactl set-sink-mute @DEFAULT_SINK@ toggle" }
            });

            var display_panel = create_setting_panel("Wyświetlacz", {
                SettingAction() { label = "Jaśniej", cmd = "brightnessctl set +5%" },
                SettingAction() { label = "Ciemniej", cmd = "brightnessctl set 5%-" }
            });

            var power_panel = create_setting_panel("Zasilanie", {
                SettingAction() { label = "Oszczędzanie", cmd = "powerprofilesctl set power-saver" },
                SettingAction() { label = "Zrównoważony", cmd = "powerprofilesctl set balanced" },
                SettingAction() { label = "Wydajność", cmd = "powerprofilesctl set performance" }
            });

            grid.attach(audio_panel, 0, 0);
            grid.attach(display_panel, 1, 0);
            grid.attach(power_panel, 0, 1);

            box.append(grid);

            var back_btn = new Button.with_label("← Powrót");
            back_btn.add_css_class("bottom-btn");
            back_btn.margin_bottom = 50;
            back_btn.halign = Align.CENTER;
            back_btn.clicked.connect(() => { stack.visible_child_name = "main"; });

            box.append(back_btn);

            stack.add_named(scrolled, "settings");
        }

        private Box create_setting_panel(string title, SettingAction[] actions) {
            var panel = new Box(Orientation.VERTICAL, 15);
            panel.add_css_class("setting-panel");

            var title_label = new Label(@"<span font='26' weight='bold'>$title</span>");
            title_label.use_markup = true;
            panel.append(title_label);

            foreach (var action in actions) {
                var btn = new Button.with_label(action.label);
                btn.add_css_class("setting-btn");
                btn.clicked.connect(() => {
                    try {
                        Process.spawn_command_line_async(action.cmd);
                    } catch {}
                });
                panel.append(btn);
            }

            return panel;
        }

        private void build_hacker_menu() {
            var menu = new Menu();
            menu.append("Switch to Plasma", "win.system_action('switchplasma')");
            menu.append("Shutdown", "win.system_action('shutdown')");
            menu.append("Restart", "win.system_action('restart')");
            menu.append("Sleep", "win.system_action('sleep')");
            menu.append("Restart Apps", "win.system_action('restartapps')");
            menu.append("Restart Sway", "win.system_action('restartsway')");

            hacker_menu = new PopoverMenu.from_model(menu);
            hacker_menu.set_parent(this);
            hacker_menu.autohide = true;

            var actions = new SimpleActionGroup();
            var action = new SimpleAction("system_action", VariantType.STRING);
            action.activate.connect((parameter) => {
                string act = parameter.get_string();
                switch (act) {
                    case "switchplasma":
                        try { Process.spawn_command_line_async("startplasma-wayland"); } catch {}
                        this.application.quit();
                        break;
                    case "shutdown":
                        try { Process.spawn_command_line_async("systemctl poweroff"); } catch {}
                        break;
                    case "restart":
                        try { Process.spawn_command_line_async("systemctl reboot"); } catch {}
                        break;
                    case "sleep":
                        try { Process.spawn_command_line_async("systemctl suspend"); } catch {}
                        break;
                    case "restartapps":
                        try { Process.spawn_command_line_async("pkill -f steam; pkill -f heroic; pkill -f hyperplay; pkill -f lutris"); } catch {}
                        break;
                    case "restartsway":
                        try { Process.spawn_command_line_async("swaymsg reload"); } catch {}
                        break;
                }
            });
            actions.add_action(action);
            this.insert_action_group("win", actions);
        }

        private void message(string text) {
            var dialog = new Dialog.with_buttons(null, this, DialogFlags.MODAL, "OK", ResponseType.OK, null);
            var content_area = dialog.get_content_area();
            var label = new Label(text);
            content_area.append(label);
            dialog.response.connect((response_id) => {
                dialog.destroy();
            });
            dialog.present();
        }
    }

    private struct LauncherInfo {
        public string name;
        public string command;
        public bool internet;
    }

    public int main(string[] args) {
        Intl.setlocale(LocaleCategory.ALL, "");
        return new Application().run(args);
    }
}
