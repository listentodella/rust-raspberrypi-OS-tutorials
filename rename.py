import os
import glob


def rename_readme_files(start_path="."):
    # 使用glob模块递归查找所有README.md文件
    for root, dirs, files in os.walk(start_path):
        for file in files:
            if file == "README.CN.md":
                # 构建完整的文件路径
                old_file_path = os.path.join(root, file)
                # 构建新的文件路径
                new_file_path = os.path.join(root, "README.md")
                # 重命名文件
                os.rename(old_file_path, new_file_path)
                print(f"Renamed: {old_file_path} to {new_file_path}")


# 从当前目录开始执行
rename_readme_files()
