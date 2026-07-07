# -*- mode: python ; coding: utf-8 -*-
import sys

a = Analysis(
    ['main.py'],
    pathex=[],
    binaries=[],
    datas=[
        ('assets/icons/duotone', 'assets/icons/duotone'),
        ('pedal_bilstm.npz', '.'),
        ('icon.ico', '.'),
    ],
    hiddenimports=(
        ['pynput.keyboard._win32', 'pynput.mouse._win32']
        if sys.platform == 'win32'
        else ['pynput.keyboard._xorg', 'pynput.mouse._xorg']
        if sys.platform.startswith('linux')
        else ['pynput.keyboard._darwin', 'pynput.mouse._darwin']
    ),
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
)

pyz = PYZ(a.pure)

if sys.platform == 'darwin':
    # macOS: directory layout feeds BUNDLE to produce a proper .app
    exe = EXE(
        pyz,
        a.scripts,
        [],
        exclude_binaries=True,
        name='HuMidi',
        debug=False,
        bootloader_ignore_signals=False,
        strip=False,
        upx=True,
        console=False,
        icon='icon.icns',
    )
    coll = COLLECT(
        exe,
        a.binaries,
        a.zipfiles,
        a.datas,
        strip=False,
        upx=True,
        upx_exclude=[],
        name='HuMidi',
    )
    app = BUNDLE(
        coll,
        name='HuMidi.app',
        icon='icon.icns',
        bundle_identifier='com.smygitt.humidi',
    )
else:
    # Windows / Linux: everything packed into a single executable
    exe = EXE(
        pyz,
        a.scripts,
        a.binaries,
        a.datas,
        [],
        name='HuMidi',
        debug=False,
        bootloader_ignore_signals=False,
        strip=False,
        upx=True,
        upx_exclude=[],
        console=False,
        icon='icon.ico',
    )
