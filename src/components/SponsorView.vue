<script lang="ts" setup>
import wechat from '../assets/wechat.jpg'
import alipay from '../assets/alipay.jpg'
</script>

<template>
  <div class="sponsor">
    <div class="hero">
      <h1>赞助作者</h1>
      <p class="subtitle">如果 FluentPlayer 对你有帮助，欢迎请作者喝杯奶茶 ☕</p>
      <p class="hint">扫码或保存图片到对应 App 即可打赏，你的支持是持续更新的动力。</p>
    </div>

    <div class="codes">
      <div class="code-card">
        <div class="code-title">微信</div>
        <img :src="wechat" alt="微信收款码" class="qr" />
        <div class="code-tip">微信扫一扫</div>
      </div>
      <div class="code-card">
        <div class="code-title">支付宝</div>
        <img :src="alipay" alt="支付宝收款码" class="qr" />
        <div class="code-tip">支付宝扫一扫</div>
      </div>
    </div>

    <p class="thanks">❤ 感谢每一位支持者的鼓励</p>
  </div>
</template>

<style scoped>
.sponsor {
  max-width: 720px;
  margin: 0 auto;
  padding: 48px 24px 64px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 36px;
}
.hero {
  text-align: center;
}
.hero h1 {
  font-size: 28px;
  font-weight: 700;
  margin: 0 0 12px;
  color: var(--fluent-text);
}
.subtitle {
  font-size: 15px;
  color: var(--fluent-text);
  margin: 0 0 6px;
}
.hint {
  font-size: 13px;
  color: var(--fluent-text-secondary);
  margin: 0;
}
.codes {
  display: flex;
  gap: 28px;
  flex-wrap: wrap;
  justify-content: center;
}
.code-card {
  background: var(--fluent-bg-elevated, rgba(255, 255, 255, 0.04));
  border: 1px solid var(--fluent-border);
  border-radius: 14px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  width: 240px;
  box-shadow: 0 6px 24px rgba(0, 0, 0, 0.18);
}
.code-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--fluent-text);
}
.qr {
  width: 200px;
  height: 200px;
  object-fit: contain;
  border-radius: 10px;
  background: #fff;
  padding: 8px;
  box-sizing: border-box;
}
.code-tip {
  font-size: 12px;
  color: var(--fluent-text-secondary);
}
.thanks {
  font-size: 14px;
  color: var(--fluent-text-secondary);
  margin: 0;
}
</style>
