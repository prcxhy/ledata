LEData Python 接口（随应用分发）
================================

本目录包含 LEData 的 Python 扩展模块（ledata_py.pyd + ledata_py.pyi），
与 LEData GUI 使用同一份 Rust 解析核心，数据口径完全一致。

使用方法（Windows，需自备 Python >= 3.10 与 numpy）:

    import sys
    sys.path.insert(0, r"<安装目录>\py-interface")
    import ledata_py

    chip, warning = ledata_py.parse_file(r"D:\data\...,Data,c1.xlsx")
    print(chip.summary())                  # 摘要（面向 Agent，无原始数组）
    pt = chip.devices[1].spectrum_at(3.5)  # 按电压匹配提取光谱

注意: 必须 sys.path.insert(0, ...)，否则环境里已安装的同名包会被优先加载。
可用 ledata_py.__file__ 验证加载的是否为本目录下的文件。
