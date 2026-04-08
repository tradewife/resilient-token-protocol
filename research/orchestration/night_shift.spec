# -*- mode: python ; coding: utf-8 -*-
#
# PyInstaller spec for night_shift.bin
# Produces a standalone binary that accepts --bridge-mode for Rust bridge integration.
#
# Build: python3 -m PyInstaller research/orchestration/night_shift.spec
# Output: dist/night_shift.bin
#
# Bridge mode reads JSON from stdin, writes JSON to stdout.
# Data files (data/ohlcv/*.parquet) are expected at the CWD at runtime.

import os

block_cipher = None

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(SPEC))))
RESEARCH_DIR = os.path.join(REPO_ROOT, 'research')
ORCH_DIR = os.path.join(RESEARCH_DIR, 'orchestration')

a = Analysis(
    [os.path.join(ORCH_DIR, 'night_shift.py')],
    pathex=[RESEARCH_DIR, REPO_ROOT],
    binaries=[],
    datas=[],
    hiddenimports=[
        'numpy',
        'pandas',
        'pandas._libs',
        'pandas._libs.tslibs.timedeltas',
        'pandas._libs.tslibs.np_datetime',
        'pandas._libs.tslibs.nattype',
        'pandas.core.arrays.masked',
        'pyarrow',
        'pyarrow.lib',
        'pyarrow.pandas_compat',
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        'tkinter',
        'matplotlib',
        'scipy',
        'seaborn',
        'plotly',
        'bokeh',
        'IPython',
        'jupyter',
        'notebook',
        'redis',
        'ccxt',
        'aiohttp',
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name='night_shift.bin',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
