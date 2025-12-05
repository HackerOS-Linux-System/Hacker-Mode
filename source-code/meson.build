project('hacker-mode', 'vala', 'c',
    version : '1.0.0',
    license : 'GPL-3.0'
)

dependencies = [
    dependency('gtk4'),
    dependency('glib-2.0'),
    dependency('gobject-2.0'),
    dependency('gio-2.0')
]

sources = files('src/main.vala')

executable('hacker-mode',
    sources,
    dependencies: dependencies,
    install: true
)
