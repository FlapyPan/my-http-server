/// 按照指定分隔符分割u8数组
pub fn split<D: AsRef<[u8]>>(data: &D, separator: impl AsRef<[u8]>) -> Vec<&[u8]> {
    let sep = separator.as_ref();
    let mut data = data.as_ref();
    // 查找分隔符位置
    let mut buf = Vec::new();
    while let Some(pos) = scan(&data, &sep) {
        // 分割数据
        let (split, rest) = data.split_at(pos);
        buf.push(split);
        // 移除分隔符
        data = rest.split_at(sep.len()).1;
    }
    buf.push(data);
    buf
}

/// 查找第一个所在位置
pub fn scan(data: impl AsRef<[u8]>, pattern: impl AsRef<[u8]>) -> Option<usize> {
    let data: &[u8] = data.as_ref();
    let pat: &[u8] = pattern.as_ref();
    if pat.len() > data.len() {
        return None;
    }
    let mut found = 0usize;
    for (i, &d) in data.iter().enumerate() {
        if d == pat[found] {
            found += 1;
            if found == pat.len() {
                return Some(i + 1 - pat.len());
            }
        } else {
            found = 0;
        }
    }
    None
}