function initReveal(){
  var els = document.querySelectorAll('.reveal,.reveal-left,.reveal-right,.reveal-fade');
  var io = new IntersectionObserver(function(entries){
    entries.forEach(function(e){
      if(e.isIntersecting){
        var d = e.target.getAttribute('data-delay') || 0;
        setTimeout(function(){ e.target.classList.add('in-view'); }, parseInt(d, 10));
        io.unobserve(e.target);
      }
    });
  }, { threshold: 0.12 });
  els.forEach(function(el){ io.observe(el); });
}

function initCarousel(){
  var root = document.querySelector('[data-carousel]');
  if(!root) return;
  var slides = Array.prototype.slice.call(root.querySelectorAll('[data-slide]'));
  var thumbs = Array.prototype.slice.call(root.querySelectorAll('[data-carousel-thumb]'));
  var prev = root.querySelector('[data-carousel-prev]');
  var next = root.querySelector('[data-carousel-next]');
  var title = root.querySelector('[data-carousel-title]');
  var copy = root.querySelector('[data-carousel-copy]');
  var active = 0;

  function setActive(index){
    active = (index + slides.length) % slides.length;
    activeCarouselIndex = active;
    var items = currentLang === 'en' ? i18n.en.carouselItems : carouselItems;
    slides.forEach(function(slide, i){
      slide.classList.toggle('is-active', i === active);
      slide.setAttribute('aria-hidden', i === active ? 'false' : 'true');
    });
    thumbs.forEach(function(thumb, i){
      thumb.setAttribute('aria-selected', i === active ? 'true' : 'false');
    });
    if(title) title.textContent = items[active].title;
    if(copy) copy.textContent = items[active].copy;
    if(thumbs[active] && thumbs[active].offsetParent) thumbs[active].scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }

  thumbs.forEach(function(thumb, i){
    thumb.addEventListener('click', function(){ setActive(i); });
  });
  if(prev) prev.addEventListener('click', function(){ setActive(active - 1); });
  if(next) next.addEventListener('click', function(){ setActive(active + 1); });
  root.addEventListener('keydown', function(e){
    if(e.key === 'ArrowLeft') setActive(active - 1);
    if(e.key === 'ArrowRight') setActive(active + 1);
  });
  setActive(0);
}

function detectOS(platformValue, agentValue){
  var detectedPlatform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || '';
  var platform = String(platformValue == null ? detectedPlatform : platformValue).toLowerCase();
  var agent = String(agentValue == null ? (navigator.userAgent || '') : agentValue).toLowerCase();
  var text = platform + ' ' + agent;
  if(/android|iphone|ipad|ipod|mobile/.test(agent) || agent.indexOf('cros') >= 0) return 'unknown';
  if(text.indexOf('mac') >= 0) return 'mac';
  if(text.indexOf('win') >= 0) return 'windows';
  if(text.indexOf('linux') >= 0 || text.indexOf('x11') >= 0) return 'linux';
  return 'unknown';
}

function fileNameFromUrl(url){
  if(!url) return '';
  try{
    return decodeURIComponent(url.split('/').pop().split('?')[0]);
  }catch(e){
    return url.split('/').pop().split('?')[0];
  }
}

function setText(selector, text){
  var el = document.querySelector(selector);
  if(el) el.textContent = text;
}

function safeStorageGet(key){
  try { return localStorage.getItem('mio-music-lang'); } catch (error) { return null; }
}

function safeStorageSet(key, value){
  try { localStorage.setItem('mio-music-lang', value); } catch (error) {}
}

function t(text){
  return currentLang === 'en' ? (translations[text] || text) : text;
}

function trAttr(value){
  return currentLang === 'en' ? (attrTranslations[value] || value) : value;
}

function detectPreferredLang(){
  var saved = safeStorageGet('mio-music-lang');
  if(saved === 'zh' || saved === 'en') return saved;
  var langs = navigator.languages && navigator.languages.length ? navigator.languages : [navigator.language || 'zh'];
  return Array.prototype.some.call(langs, function(lang){
    return String(lang || '').toLowerCase().indexOf('en') === 0;
  }) ? 'en' : 'zh';
}

function captureOriginalText(){
  var nodes = [];
  function visit(node){
    if(!node) return;
    if(node.nodeType === 3){
      if(node.nodeValue && node.nodeValue.trim() && /[\u4e00-\u9fff]/.test(node.nodeValue)){
        nodes.push(node);
      }
      return;
    }
    if(node.nodeType !== 1) return;
    var tag = node.tagName && node.tagName.toLowerCase();
    if(tag === 'script' || tag === 'style') return;
    Array.prototype.forEach.call(node.childNodes || [], visit);
  }
  visit(document.body);
  nodes.forEach(function(node){
    if(!originalText.has(node)) originalText.set(node, node.nodeValue);
  });
  document.querySelectorAll('[alt],[aria-label],[title]').forEach(function(el){
    ['alt', 'aria-label', 'title'].forEach(function(attr){
      if(!el.hasAttribute(attr)) return;
      var value = el.getAttribute(attr);
      if(!value || !/[\u4e00-\u9fff]/.test(value)) return;
      var attrs = originalAttrs.get(el) || {};
      attrs[attr] = attrs[attr] || value;
      originalAttrs.set(el, attrs);
    });
  });
}

function setMeta(selector, value){
  var el = document.querySelector(selector);
  if(el && value) el.setAttribute('content', value);
}

function updateLanguageToggle(){
  document.querySelectorAll('[data-lang-toggle]').forEach(function(button){
    var isActive = button.dataset.langToggle === currentLang;
    button.setAttribute('aria-pressed', isActive ? 'true' : 'false');
    button.classList.toggle('bg-cyan-400', isActive);
    button.classList.toggle('text-[#0B0F19]', isActive);
    button.classList.toggle('text-gray-400', !isActive);
  });
}

function updateStaticText(){
  originalText.forEach(function(original, node){
    node.nodeValue = currentLang === 'en' ? (translations[original.trim()] || original) : original;
  });
  originalAttrs.forEach(function(attrs, el){
    Object.keys(attrs).forEach(function(attr){
      el.setAttribute(attr, trAttr(attrs[attr]));
    });
  });
  document.documentElement.lang = currentLang === 'en' ? 'en' : 'zh-CN';
  var meta = currentLang === 'en' ? i18n.en : zhMeta;
  document.title = meta.title;
  setMeta('meta[name="description"]', currentLang === 'en' ? meta.metaDescription : meta.description);
  setMeta('meta[property="og:title"]', meta.ogTitle);
  setMeta('meta[property="og:description"]', meta.ogDescription);
  setMeta('meta[property="og:locale"]', meta.ogLocale);
  setMeta('meta[name="twitter:title"]', meta.ogTitle);
  setMeta('meta[name="twitter:description"]', meta.ogDescription);
}

function updateCarouselLanguage(){
  var title = document.querySelector('[data-carousel-title]');
  var copy = document.querySelector('[data-carousel-copy]');
  var items = currentLang === 'en' ? i18n.en.carouselItems : carouselItems;
  if(title) title.textContent = items[activeCarouselIndex].title;
  if(copy) copy.textContent = items[activeCarouselIndex].copy;
}

function setLanguage(lang, persist){
  currentLang = lang === 'en' ? 'en' : 'zh';
  if(persist) safeStorageSet('mio-music-lang', currentLang);
  updateStaticText();
  updateCarouselLanguage();
  updateLanguageToggle();
  initDownloads();
}

function initLanguage(){
  currentLang = detectPreferredLang();
  captureOriginalText();
  document.querySelectorAll('[data-lang-toggle]').forEach(function(button){
    button.addEventListener('click', function(){ setLanguage(button.dataset.langToggle, true); });
  });
  window.__mioMusicSetLang = function(lang){ setLanguage(lang, true); };
  setLanguage(currentLang, false);
}
