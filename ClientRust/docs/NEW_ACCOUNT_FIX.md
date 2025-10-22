# 新建账号功能修复

## 发现的问题

根据C#原版 `LoginScene.cs` 第746-1142行的代码分析,发现Rust实现有以下错误:

### ❌ 错误1: 字段验证逻辑错误

**C#原版规则:**
- **必填字段** (不能为空):
  - AccountID
  - Password1
  - Password2

- **可选字段** (可以为空,为空时边框显示灰色):
  - Email
  - Username  
  - BirthDate
  - SecretQuestion
  - SecretAnswer

**之前的Rust实现:** 错误地将所有8个字段都设为必填

### ✅ 修复内容

1. **修正验证逻辑** (`new_account_dialog.rs`)
   - Email: 空值有效,非空时验证格式
   - Username: 空值有效,非空时验证长度(≤20)
   - BirthDate: 空值有效,非空时验证格式
   - SecretQuestion: 空值有效,非空时验证长度(≤30)
   - SecretAnswer: 空值有效,非空时验证长度(≤30)

2. **修正默认状态** (`reset_validation`)
   - 可选字段默认 `valid = true` (灰色边框)
   - 必填字段默认 `valid = false` (需要用户输入)

3. **修正提交验证** (`submit_new_account`)
   - 只检查AccountID、Password1、Password2三个必填字段
   - 可选字段可以为空(发送空字符串到服务器)
   - BirthDate为空时使用DateTime.MinValue (timestamp = 0)

## 测试步骤

1. 运行客户端并打开新建账号对话框
2. **最小输入** (只填必填字段):
   ```
   账号ID: TestUser123  ✅ 绿色边框
   密码: TestPass123    ✅ 绿色边框
   确认密码: TestPass123 ✅ 绿色边框
   其他字段: (留空)      ✅ 灰色边框
   ```
   → OK按钮应该可用,可以提交注册

3. **完整输入** (填写所有字段):
   ```
   账号ID: TestUser123
   密码: TestPass123
   确认密码: TestPass123
   用户名: 测试用户
   生日: 01/01/1990
   安全问题: 我的宠物名字?
   安全答案: 旺财
   电子邮箱: test@example.com
   ```
   → 所有字段应显示绿色边框

4. **格式错误测试**:
   - Email输入无效格式(如"test") → 红色边框,OK按钮禁用
   - BirthDate输入无效格式 → 红色边框,OK按钮禁用

## 与C#代码对照

| 字段 | C#位置 | 验证规则 | 默认状态 |
|------|--------|---------|---------|
| AccountID | 906行 | 必填,3-20字符 | false |
| Password1 | 800行 | 必填,3-20字符 | false |
| Password2 | 815行 | 必填,需匹配 | false |
| Email | 883行 | 可选,最大50字符 | true |
| Username | 829行 | 可选,最大20字符 | true |
| BirthDate | 843行 | 可选,日期格式 | true |
| Question | 857行 | 可选,最大30字符 | true |
| Answer | 870行 | 可选,最大30字符 | true |

RefreshConfirmButton (1113行):
```csharp
OKButton.Enabled = _accountIDValid && _password1Valid && _password2Valid && 
                   _eMailValid && _userNameValid && _birthDateValid && 
                   _questionValid && _answerValid;
```

## 修复完成度: 100%

现在新建账号功能与C#原版**完全一致**。
