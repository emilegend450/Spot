import sys

def main():
    filename = 'src/api/spotify.rs'
    with open(filename, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # Find the line index of '#[cfg(test)]'
    cfg_idx = None
    for i, line in enumerate(lines):
        if line.strip() == '#[cfg(test)]':
            cfg_idx = i
            break
    if cfg_idx is None:
        print('ERROR: #[cfg(test)] not found')
        sys.exit(1)

    # Find the line index of 'impl Spotify {'
    impl_start = None
    for i, line in enumerate(lines):
        if line.strip() == 'impl Spotify {':
            impl_start = i
            break
    if impl_start is None:
        print('ERROR: impl Spotify { not found')
        sys.exit(1)

    # Find the matching closing brace for the impl block
    brace_count = 0
    found_open = False
    impl_end = None
    for i in range(impl_start, len(lines)):
        stripped = lines[i].strip()
        if stripped.startswith('{'):
            brace_count += 1
            found_open = True
        elif stripped == '}':
            brace_count -= 1
            if brace_count < 0:
                print(f'ERROR: Unbalanced brace at line {i+1}')
                sys.exit(1)
            if found_open and brace_count == 0:
                impl_end = i
                break
    if impl_end is None:
        print('ERROR: Could not find matching closing brace for impl Spotify')
        sys.exit(1)

    # Now we have:
    #   impl_start: line index of 'impl Spotify {'
    #   impl_end: line index of the closing brace of impl Spotify
    #   cfg_idx: line index of '#[cfg(test)]'

    # We know impl_end < cfg_idx (because the impl block comes before the TokenInfo struct and then the test module)

    # Define the structs to insert before '#[cfg(test)]'
    structs = [
        '\n',
        '// User profile\n',
        '#[derive(Deserialize, Debug, Clone)]\n',
        'pub struct CurrentUser {\n',
        '    pub display_name: String,\n',
        '    pub email: String,\n',
        '    pub id: String,\n',
        '    pub product: String,\n',
        '}\n',
        '\n',
        '// Simplified playlist for listing\n',
        '#[derive(Deserialize, Debug, Clone)]\n',
        'pub struct SimplePlaylist {\n',
        '    pub name: String,\n',
        '    pub id: String,\n',
        '    pub images: Vec<PlaylistImage>,\n',
        '}\n',
        '\n',
        '#[derive(Deserialize, Debug, Clone)]\n',
        'pub struct PlaylistImage {\n',
        '    pub url: String,\n',
        '}\n',
        '\n',
        '#[derive(Deserialize, Debug, Clone)]\n',
        'pub struct Page<T> {\n',
        '    pub href: String,\n',
        '    pub items: Vec<T>,\n',
        '    pub limit: i32,\n',
        '    pub next: Option<String>,\n',
        '    pub offset: i32,\n',
        '    pub previous: Option<String>,\n',
        '    pub total: i32,\n',
        '}\n',
        '\n',
    ]

    # Define the methods to insert before the closing brace of impl Spotify (i.e., at impl_end)
    methods = [
        '\n',
        '    /// Generic GET request to the Spotify API\n',
        '    pub async fn get<T: for<\\'de> Deserialize<\\'de>>(&self, endpoint: &str) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {\n',
        '        let token = self.token.lock().await;\n',
        '        let token = token.as_ref()\n',
        '            .ok_or_else(|| "No token available".to_string())?;\n',
        '        let url = format!("https://api.spotify.com/v1/{}", endpoint);\n',
        '        let response = self.http_client\n',
        '            .get(&url)\n',
        '            .bearer_access_token(token.access_token.as_str())\n',
        '            .send()\n',
        '            .await?\n',
        '            .error_for_status()?;\n',
        '        let json = response.json().await?;\n',
        '        Ok(json)\n',
        '    }\n',
        '\n',
        '    /// Get the current user\\'s profile\n',
        '    pub async fn current_user(&self) -> Result<CurrentUser, Box<dyn std::error::Error + Send + Sync>> {\n',
        '        self.get("me").await\n',
        '    }\n',
        '\n',
        '    /// Get the current user\\'s playlists\n',
        '    pub async fn user_playlists(&self, limit: usize = 20) -> Result<Page<SimplePlaylist>, Box<dyn std::error::Error + Send + Sync>> {\n',
        '        self.get(format!("me/playlists?limit={}", limit)).await\n',
        '    }\n',
    ]

    # Build new lines:
    new_lines = []
    # 1. Lines from start to impl_start (exclusive)
    new_lines.extend(lines[:impl_start])
    # 2. Lines from impl_start to impl_end (exclusive) -> the impl block without its closing brace
    new_lines.extend(lines[impl_start:impl_end])
    # 3. Insert the methods
    new_lines.extend(methods)
    # 4. Add the closing brace of impl block
    new_lines.append(lines[impl_end])
    # 5. Lines from impl_end+1 to cfg_idx (exclusive) -> the part between impl block and test module
    new_lines.extend(lines[impl_end+1:cfg_idx])
    # 6. Insert the structs
    new_lines.extend(structs)
    # 7. Lines from cfg_idx to end
    new_lines.extend(lines[cfg_idx:])

    with open(filename, 'w', encoding='utf-8') as f:
        f.writelines(new_lines)
    print('Successfully updated src/api/spotify.rs')

if __name__ == '__main__':
    main()
