ToolCheckout - portable build
=============================

1. Copy this whole folder (ToolCheckout.exe AND the file named "portable")
   to a place you can write to, e.g. a USB stick or Documents\ToolCheckout.
2. Double-click ToolCheckout.exe. No installation and no administrator
   rights are needed.

Because the "portable" marker file sits next to the exe, ToolCheckout keeps
ALL of its data (database, backups, WebView cache) in a "data" folder beside
the exe. Delete the "portable" file to use the normal per-user location
(%LOCALAPPDATA%\ToolCheckout) instead.

Unsigned build: if Windows SmartScreen says "Windows protected your PC",
click "More info", then "Run anyway".

ToolCheckout needs the Microsoft Edge WebView2 Runtime, which is already part
of Windows 10 (current updates) and Windows 11.
