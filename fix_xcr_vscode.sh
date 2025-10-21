#!/bin/bash
#
# VS Code Remote-SSH 修复脚本 - xcr 用户
# 用途：修复 xcr 用户无法使用完整 VS Code Server 的问题
#
# 使用方法：
#   1. 上传此脚本到服务器
#   2. chmod +x fix_xcr_vscode.sh
#   3. sudo ./fix_xcr_vscode.sh
#

set -e

echo "=========================================="
echo "VS Code Remote-SSH 修复脚本"
echo "=========================================="
echo ""

# 检查是否以 root 运行
if [ "$EUID" -ne 0 ]; then 
    echo "❌ 请使用 sudo 运行此脚本"
    echo "   sudo $0"
    exit 1
fi

USERNAME="xcr"
HOME_DIR="/home/$USERNAME"

echo "📋 步骤 1: 检查用户信息"
if ! id "$USERNAME" &>/dev/null; then
    echo "❌ 用户 $USERNAME 不存在"
    exit 1
fi
echo "✅ 用户 $USERNAME 存在"
echo ""

echo "📋 步骤 2: 检查 home 目录"
if [ ! -d "$HOME_DIR" ]; then
    echo "❌ Home 目录 $HOME_DIR 不存在"
    exit 1
fi
echo "✅ Home 目录存在: $HOME_DIR"
echo ""

echo "📋 步骤 3: 修复 home 目录权限"
chown -R $USERNAME:$USERNAME $HOME_DIR
chmod 755 $HOME_DIR
echo "✅ Home 目录权限已修复"
echo ""

echo "📋 步骤 4: 检查磁盘空间"
AVAILABLE=$(df -BM $HOME_DIR | tail -1 | awk '{print $4}' | sed 's/M//')
echo "   可用空间: ${AVAILABLE}MB"
if [ "$AVAILABLE" -lt 500 ]; then
    echo "⚠️  警告: 可用空间不足 500MB，可能影响安装"
else
    echo "✅ 磁盘空间充足"
fi
echo ""

echo "📋 步骤 5: 清理旧的 VS Code Server"
sudo -u $USERNAME bash <<EOF
    if [ -d "$HOME_DIR/.vscode-server" ]; then
        echo "   发现旧的 .vscode-server 目录，正在清理..."
        rm -rf $HOME_DIR/.vscode-server
        echo "   ✅ 已清理"
    else
        echo "   没有旧的 .vscode-server 目录"
    fi
    
    if [ -d "$HOME_DIR/.vscode-server-insiders" ]; then
        echo "   发现旧的 .vscode-server-insiders 目录，正在清理..."
        rm -rf $HOME_DIR/.vscode-server-insiders
        echo "   ✅ 已清理"
    fi
EOF
echo ""

echo "📋 步骤 6: 创建新的 VS Code Server 目录"
sudo -u $USERNAME bash <<EOF
    mkdir -p $HOME_DIR/.vscode-server/bin
    mkdir -p $HOME_DIR/.vscode-server/extensions
    chmod 700 $HOME_DIR/.vscode-server
    echo "   ✅ 目录已创建"
EOF
echo ""

echo "📋 步骤 7: 检查 /tmp 目录权限"
TMP_PERMS=$(stat -c "%a" /tmp)
if [ "$TMP_PERMS" != "1777" ]; then
    echo "   修复 /tmp 权限..."
    chmod 1777 /tmp
    echo "   ✅ /tmp 权限已修复"
else
    echo "   ✅ /tmp 权限正确"
fi
echo ""

echo "📋 步骤 8: 检查 Shell 配置"
sudo -u $USERNAME bash <<EOF
    # 确保 .bashrc 和 .profile 存在
    if [ ! -f "$HOME_DIR/.bashrc" ]; then
        cp /etc/skel/.bashrc $HOME_DIR/.bashrc 2>/dev/null || touch $HOME_DIR/.bashrc
        echo "   ✅ 已创建 .bashrc"
    fi
    
    if [ ! -f "$HOME_DIR/.profile" ]; then
        cp /etc/skel/.profile $HOME_DIR/.profile 2>/dev/null || touch $HOME_DIR/.profile
        echo "   ✅ 已创建 .profile"
    fi
EOF
echo ""

echo "📋 步骤 9: 检查网络连接"
if curl -sSf -m 5 https://update.code.visualstudio.com > /dev/null 2>&1; then
    echo "✅ 可以访问 VS Code 更新服务器"
else
    echo "⚠️  警告: 无法访问 VS Code 更新服务器，可能需要配置代理"
fi
echo ""

echo "📋 步骤 10: 最终权限检查"
echo "   $HOME_DIR 权限:"
ls -ld $HOME_DIR

if [ -d "$HOME_DIR/.vscode-server" ]; then
    echo ""
    echo "   $HOME_DIR/.vscode-server 权限:"
    ls -ld $HOME_DIR/.vscode-server
fi
echo ""

echo "=========================================="
echo "✅ 修复完成！"
echo "=========================================="
echo ""
echo "📝 后续操作："
echo "   1. 在客户端 VS Code 中按 Ctrl+Shift+P"
echo "   2. 运行 'Remote-SSH: Kill VS Code Server on Host'"
echo "   3. 重新连接到服务器"
echo ""
echo "   如果还有问题，请查看详细日志："
echo "   tail -f ~/.vscode-server/.*.log"
echo ""

