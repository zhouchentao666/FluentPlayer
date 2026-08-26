function init(){
  initReveal();
  initCarousel();
  initLanguage();
}

if(document.readyState === 'loading'){
  document.addEventListener('DOMContentLoaded', init);
}else{
  init();
}
