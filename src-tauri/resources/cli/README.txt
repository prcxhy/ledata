LEData 命令行接口（随应用分发）
================================

本目录包含 ledata-cli.exe，与 LEData GUI 使用同一份 Rust 解析核心。

常用命令:

    ledata-cli.exe <xlsx文件或目录>                    # 摘要 JSON（面向 Agent）
    ledata-cli.exe <...> --voltage 3.5                # 摘要附加各器件 3.5V 处光谱段
    ledata-cli.exe <...> --voltage 3.5 --site 1A      # 仅指定器件
    ledata-cli.exe <...> --full                       # 全量数据（体积大）
    ledata-cli.exe <...> --csv performance            # GUI 同口径 TSV（Origin 可直贴）
    ledata-cli.exe <...> --csv spectra --voltage 3.5  # GUI 同口径光谱 TSV

退出码: 0 成功 / 1 数据类失败 / 2 用法错误
