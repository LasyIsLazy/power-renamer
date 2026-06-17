export const QUICK_RULES = [
  {
    id: 'quick-lowercase',
    name: '转小写',
    description: '将文件名转为小写',
    script: `function rename() {
  return { [__filePath]: __fileName.toLowerCase() };
}`,
  },
  {
    id: 'quick-spaces',
    name: '空格转下划线',
    description: '将空格替换为下划线',
    script: `function rename() {
  return { [__filePath]: __fileName.replace(/\\s+/g, '_') };
}`,
  },
  {
    id: 'quick-prefix',
    name: '添加前缀',
    description: '在文件名前添加 IMG_ 前缀',
    script: `function rename() {
  return { [__filePath]: 'IMG_' + __fileName };
}`,
  },
  {
    id: 'quick-sequence',
    name: '序号命名',
    description: '按文件列表顺序添加序号前缀',
    script: `function rename() {
  var files = __utils.fs.readDirFilesRecursive(__filePath);
  var result = {};
  for (var i = 0; i < files.length; i++) {
    var filePath = files[i];
    if (__utils.path.isFile(filePath)) {
      var ext = __utils.path.extname(filePath);
      var base = __utils.path.basename(filePath, ext);
      result[filePath] = String(i + 1).padStart(3, '0') + '_' + base + ext;
    }
  }
  if (Object.keys(result).length === 0) {
    return { [__filePath]: __fileName };
  }
  return result;
}`,
  },
  {
    id: 'quick-replace',
    name: '替换文本',
    description: '将文件名中的 old 替换为 new（通过参数配置）',
    parameters: [
      { name: 'oldText', type: 'string', default: 'old', description: '要替换的文本', required: true },
      { name: 'newText', type: 'string', default: 'new', description: '替换为', required: false },
    ],
    script: `function rename() {
  var oldText = __params && __params.oldText ? __params.oldText : 'old';
  var newText = __params && __params.newText ? __params.newText : 'new';
  return { [__filePath]: __fileName.split(oldText).join(newText) };
}`,
  },
]
