# THIS FILE IS AUTO-GENERATED. DO NOT MODIFY!!

# Copyright 2020-2023 Tauri Programme within The Commons Conservancy
# SPDX-License-Identifier: Apache-2.0
# SPDX-License-Identifier: MIT

-keep class com.vant.Mio.Music.* {
  native <methods>;
}

-keep class com.vant.Mio.Music.WryActivity {
  public <init>(...);

  void setWebView(com.vant.Mio.Music.RustWebView);
  java.lang.Class getAppClass(...);
  int getId();
  java.lang.String getVersion();
  int startActivity(...);
}

-keep class com.vant.Mio.Music.Ipc {
  public <init>(...);

  @android.webkit.JavascriptInterface public <methods>;
}

-keep class com.vant.Mio.Music.RustWebView {
  public <init>(...);

  void loadUrlMainThread(...);
  void loadHTMLMainThread(...);
  void evalScript(...);
}

-keep class com.vant.Mio.Music.RustWebChromeClient,com.vant.Mio.Music.RustWebViewClient {
  public <init>(...);
}
