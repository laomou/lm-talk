export default {
  nav: {
    chat: '聊天', contacts: '通讯录', me: '我', openChat: '打开聊天', openContacts: '打开通讯录', openMe: '打开我的设置',
  },
  auth: {
    preparingTitle: '正在准备 LM Talk', preparingDescription: '正在加载安全组件…',
    login: '登录', register: '注册', import: '导入', registerTitle: '创建身份', importTitle: '导入身份',
    loginDescription: '选择本机身份并输入提示词继续。', registerDescription: '创建身份后，请立即完成备份。', importDescription: '粘贴身份文本并输入对应提示词。',
    passphrase: '提示词', loginPassphrase: '登录提示词', registerPassphrase: '注册提示词', importPassphrase: '导入身份提示词',
    enterPassphrase: '输入你的提示词', setPassphrase: '设置你的提示词', enterImportPassphrase: '输入身份对应提示词',
    selectIdentity: '选择身份', noLocalIdentityTitle: '还没有本机身份', noLocalIdentityDescription: '注册新身份，或导入已有身份后再登录。',
    noIdentity: '还没有身份？', registerLink: '注册', importLink: '导入', backToLogin: '返回登录',
    identityCreated: '身份已创建', identityCreatedDescription: '身份已保存在本机。请先完成备份，再返回登录。', downloadIdentity: '下载身份', verifyImport: '验证导入', goLogin: '去登录',
    passphraseNotice: '提示词不会上传或找回；注册后请下载身份文件。', importNotice: '导入需要身份文本和对应提示词；提示词错误或丢失时无法恢复。',
    identityText: '身份文本', importIdentityText: '导入身份文本', pasteIdentityText: '粘贴导出的身份文本',
  },
  me: {
    profile: '个人资料', backup: '身份备份', security: '安全与设备', sync: '同步与安全', settings: '设置', about: '关于', logout: '退出登录',
    language: '语言', languageDescription: '选择界面显示语言。修改后立即生效，并仅保存在本机。',
  },
  language: { zhCN: '简体中文', enUS: 'English' },
  common: { save: '保存', cancel: '取消', close: '关闭', backToMe: '返回我' },
} as const
