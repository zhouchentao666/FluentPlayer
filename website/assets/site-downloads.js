function setDownloadStatus(message, tone){
  var status = document.querySelector('#download-status');
  if(!status) return;
  status.textContent = t(message);
  status.className = 'text-xs mb-6 min-h-[1.25rem]';
  if(tone === 'ready') status.classList.add('text-cyan-300');
  else if(tone === 'fallback') status.classList.add('text-yellow-300');
  else status.classList.add('text-gray-500');
}

function choosePlatform(platforms, os){
  if(os === 'mac') return null;
  var keys = platformPriority[os] || [];
  for(var i = 0; i < keys.length; i += 1){
    var item = platforms && platforms[keys[i]];
    if(item && item.url) return { url: item.url, name: fileNameFromUrl(item.url) };
  }
  return null;
}

function chooseBestAsset(platforms, assets, os){
  var direct = choosePlatform(platforms, os);
  if(direct) return direct;
  return chooseAsset(assets || [], os);
}

function chooseAsset(assets, os){
  if(os === 'mac') return null;
  var matchers = assetMatchers[os] || [];
  for(var i = 0; i < matchers.length; i += 1){
    for(var j = 0; j < assets.length; j += 1){
      var asset = assets[j];
      if(matchers[i].test(asset.name || '') && asset.browser_download_url){
        return { url: asset.browser_download_url, name: asset.name };
      }
    }
  }
  return null;
}

function normalizeVersion(version){
  if(!version) return '';
  return String(version).charAt(0).toLowerCase() === 'v' ? String(version) : 'v' + version;
}

function setVersionLabels(version){
  var normalized = normalizeVersion(version) || fallbackVersion;
  setText('#latest-version-stat', normalized);
  setText('#latest-version-label', normalized);
}

function applyDownloadOption(option, asset, isRecommended){
  if(!option) return;
  option.classList.toggle('border-purple-500/40', !!isRecommended);
  option.classList.toggle('bg-purple-500/10', !!isRecommended);
  option.setAttribute('aria-label', trAttr(isRecommended ? '推荐下载 ' + osLabels[option.dataset.os] + ' 版本' : '下载 ' + osLabels[option.dataset.os] + ' 版本'));
  var name = option.querySelector('[data-download-name]');
  var badge = option.querySelector('[data-recommended-badge]');
  var fallbackName = option.dataset.downloadFallbackName;
  if(!fallbackName && name){
    fallbackName = name.textContent;
    option.dataset.downloadFallbackName = fallbackName;
  }
  if(badge) badge.classList.toggle('hidden', !isRecommended);
  if(!asset){
    option.href = releasesUrl;
    if(name) name.textContent = fallbackName;
    return;
  }
  option.href = asset.url;
  if(name) name.textContent = asset.name || fileNameFromUrl(asset.url);
}

async function fetchJsonFromUrls(urls){
  var lastError = null;
  for(var i = 0; i < urls.length; i += 1){
    try{
      var response = await fetch(urls[i], { headers: { Accept: 'application/json' }, cache: 'no-cache' });
      if(!response.ok) throw new Error('HTTP ' + response.status);
      return await response.json();
    }catch(error){
      // 这里捕获的失败可能包含 CORS 跨源错误，浏览器控制台仍可能打印拦截日志，
      // 属于正常兜底逻辑：继续尝试下一个源或回退到 GitHub API。
      lastError = error;
    }
  }
  throw lastError || new Error('No latest.json URL available');
}

async function fetchReleaseFromGitHubApi(){
  // api.github.com 会返回 CORS 头，浏览器可正常跨源读取。
  var response = await fetch(latestReleaseApi, {
    headers: { Accept: 'application/vnd.github+json' },
    cache: 'no-cache'
  });
  if(!response.ok) throw new Error('HTTP ' + response.status);
  var release = await response.json();
  return {
    source: 'GitHub API',
    version: release.tag_name || release.name,
    assets: Array.isArray(release.assets) ? release.assets : []
  };
}

async function getDownloadManifest(){
  // 默认走支持 CORS 的 GitHub API；若仓库默认分支存在 latest.json，
  // 再通过 CORS 友好的镜像（jsDelivr / raw.githubusercontent）读取作为增强。
  try{
    return await fetchReleaseFromGitHubApi();
  }catch(apiError){
    try{
      var manifest = await fetchJsonFromUrls(latestJsonUrls);
      return { source: 'latest.json', version: manifest.version, platforms: manifest.platforms || {} };
    }catch(jsonError){
      throw apiError;
    }
  }
}

function initDownloads(){
  var requestId = ++downloadRequestId;
  currentOS = detectOS();
  var primary = document.querySelector('#primary-download');
  var primaryLabel = document.querySelector('[data-primary-download-label]');
  var options = Array.prototype.slice.call(document.querySelectorAll('[data-download-option]'));
  osLabels = currentLang === 'en' ? i18n.en.osLabels : { mac: 'macOS', windows: 'Windows', linux: 'Linux', unknown: '未知系统' };

  setVersionLabels(fallbackVersion);
  setDownloadStatus('正在读取最新安装包列表...');
  setText('#detected-os', currentOS === 'unknown' ? t('未能自动识别，请手动选择') : (currentLang === 'en' ? osLabels[currentOS] + ' device' : osLabels[currentOS] + ' 设备'));
  if(primary){
    primary.href = releasesUrl;
    if(primaryLabel) primaryLabel.textContent = currentOS === 'unknown' ? t('打开下载页') : (currentLang === 'en' ? 'Download ' + osLabels[currentOS] : '下载 ' + osLabels[currentOS] + ' 版');
  }
  options.forEach(function(option){
    applyDownloadOption(option, null, option.dataset.os === currentOS);
  });

  getDownloadManifest().then(function(manifest){
    if(requestId !== downloadRequestId) return;
    var version = normalizeVersion(manifest.version);
    setVersionLabels(version);

    var selectedAsset = null;
    options.forEach(function(option){
      var os = option.dataset.os;
      var asset = manifest.platforms ? choosePlatform(manifest.platforms, os) : chooseAsset(manifest.assets || [], os);
      var isRecommended = os === currentOS;
      applyDownloadOption(option, asset, isRecommended);
      if(isRecommended && asset) selectedAsset = asset;
    });

    var primaryAsset = chooseBestAsset(manifest.platforms, manifest.assets, currentOS) || selectedAsset;
    if(primary && primaryAsset){
      primary.href = primaryAsset.url;
      if(primaryLabel) primaryLabel.textContent = currentLang === 'en' ? 'Download ' + osLabels[currentOS] : '下载 ' + osLabels[currentOS] + ' 版';
      latestAssetName = primaryAsset.name;
      setDownloadStatus((currentLang === 'en' ? 'Matched latest installer: ' : '已匹配最新安装包：') + primaryAsset.name, 'ready');
    }else if(currentOS === 'mac'){
      setDownloadStatus(currentLang === 'en' ? 'macOS installers vary by chip architecture. Choose the matching build on GitHub Releases.' : 'macOS 安装包按芯片架构区分，请在 GitHub Releases 页面选择对应版本。', 'fallback');
    }else{
      setDownloadStatus(currentLang === 'en' ? 'No direct installer was found for this system. Keeping the GitHub Releases page.' : '没有找到当前系统的直接安装包，已保留 GitHub Release 下载页。', 'fallback');
    }
  }).catch(function(){
    if(requestId !== downloadRequestId) return;
    setDownloadStatus(currentLang === 'en' ? 'Could not load the latest installer list. Keeping the GitHub Releases page.' : '无法读取最新安装包列表，已保留 GitHub Release 下载页。', 'fallback');
    setVersionLabels(fallbackVersion);
  });
}
