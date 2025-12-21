import customtkinter as ctk
from launchers import launch_app, system_action
from settings import create_settings_window
from utils import setup_language, get_text, log

ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("dark-blue")

def toggle_hacker_menu(frame, root):
    if frame.winfo_ismapped():
        frame.pack_forget()
    else:
        frame.pack(side='bottom', fill='x', pady=10)

def create_main_window():
    root = ctk.CTk()
    root.attributes('-fullscreen', True)
    root.title(get_text('title'))
    root.configure(bg='#1A1A1A')

    # Title
    title_label = ctk.CTkLabel(root, text=get_text('title'), font=ctk.CTkFont(size=48, weight='bold'), text_color='#0FF')
    title_label.pack(pady=40)

    # Launch buttons grid
    launch_frame = ctk.CTkFrame(root, fg_color='transparent')
    launch_frame.pack(pady=20)

    apps = [
        {'name': 'steam', 'image': 'https://store.steampowered.com/favicon.ico'},
        {'name': 'heroic', 'image': 'https://www.heroicgameslauncher.com/favicon.ico'},
        {'name': 'hyperplay', 'image': 'https://cdn-icons-png.flaticon.com/512/5968/5968778.png'},
        {'name': 'lutris', 'image': 'https://lutris.net/static/images/logo.png'}
    ]
    for i, app in enumerate(apps):
        btn = ctk.CTkButton(
            launch_frame,
            text=get_text(app['name']),
            command=lambda a=app['name']: launch_app(a, root),
            width=200,
            height=200,
            fg_color='#2D2D2D',
            hover_color='#3D3D3D',
            corner_radius=20,
            font=ctk.CTkFont(size=18)
        )
        # Note: CustomTkinter doesn't support images directly, so text only for simplicity. For images, need PIL or similar.
        btn.grid(row=0, column=i, padx=40, pady=20)

    # Bottom frame
    bottom_frame = ctk.CTkFrame(root, fg_color='transparent')
    bottom_frame.pack(side='bottom', fill='x', pady=20, padx=20)

    settings_btn = ctk.CTkButton(
        bottom_frame,
        text=get_text('settings'),
        command=lambda: create_settings_window(root),
        width=150,
        fg_color='#2D2D2D',
        hover_color='#3D3D3D',
        corner_radius=10
    )
    settings_btn.pack(side='left', padx=20)

    hacker_menu_btn = ctk.CTkButton(
        bottom_frame,
        text=get_text('hacker_menu'),
        command=lambda: toggle_hacker_menu(hacker_menu_frame, root),
        width=150,
        fg_color='#2D2D2D',
        hover_color='#3D3D3D',
        corner_radius=10
    )
    hacker_menu_btn.pack(side='left', padx=20)

    # Hacker menu
    hacker_menu_frame = ctk.CTkFrame(root, fg_color='#2D2D2D', corner_radius=10)
    hacker_menu_frame.pack_forget()

    menu_actions = ['switch_to_plasma', 'shutdown', 'restart', 'sleep', 'restart_apps', 'restart_sway']
    for act in menu_actions:
        btn = ctk.CTkButton(
            hacker_menu_frame,
            text=get_text(act),
            command=lambda a=act: system_action(a, root),
            fg_color='transparent',
            hover_color='#3D3D3D',
            anchor='w'
        )
        btn.pack(fill='x', pady=5, padx=10)

    return root

if __name__ == "__main__":
    setup_language()
    main_window = create_main_window()
    main_window.mainloop()
