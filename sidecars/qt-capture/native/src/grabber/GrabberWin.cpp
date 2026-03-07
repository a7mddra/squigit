/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include "ScreenGrabber.h"
#include <QGuiApplication>
#include <QScreen>

#include <gdiplus.h>
#include <windows.h>

#pragma comment(lib, "Gdiplus.lib")
#pragma comment(lib, "User32.lib")
#pragma comment(lib, "Gdi32.lib")

#include <ShellScalingApi.h>
#pragma comment(lib, "Shcore.lib")

using namespace Gdiplus;

struct MonitorData {
  std::vector<CapturedFrame> frames;
  int indexCounter = 0;
};

class ScreenGrabberWin : public ScreenGrabber {
public:
  ScreenGrabberWin(QObject *parent = nullptr) : ScreenGrabber(parent) {
    GdiplusStartupInput gdiplusStartupInput;
    GdiplusStartup(&m_gdiplusToken, &gdiplusStartupInput, NULL);
  }

  ~ScreenGrabberWin() { GdiplusShutdown(m_gdiplusToken); }

  std::vector<CapturedFrame> captureAll() override {
    MonitorData data;

    HDC hdc = GetDC(NULL);
    EnumDisplayMonitors(hdc, NULL, MonitorEnumProc,
                        reinterpret_cast<LPARAM>(&data));
    ReleaseDC(NULL, hdc);

    ScreenGrabber::sortLeftToRight(data.frames);

    for (size_t i = 0; i < data.frames.size(); i++) {
      data.frames[i].index = static_cast<int>(i);
    }

    return data.frames;
  }

private:
  ULONG_PTR m_gdiplusToken;

  static BOOL CALLBACK MonitorEnumProc(HMONITOR hMonitor, HDC hdcMonitor,
                                       LPRECT lprcMonitor, LPARAM dwData) {
    MonitorData *data = reinterpret_cast<MonitorData *>(dwData);

    MONITORINFOEXW mi;
    mi.cbSize = sizeof(MONITORINFOEXW);
    if (!GetMonitorInfoW(hMonitor, &mi))
      return TRUE;

    QRect geometry(mi.rcMonitor.left, mi.rcMonitor.top,
                   mi.rcMonitor.right - mi.rcMonitor.left,
                   mi.rcMonitor.bottom - mi.rcMonitor.top);

    int w = geometry.width();
    int h = geometry.height();

    HDC hScreenDC = GetDC(NULL);
    HDC hMemoryDC = CreateCompatibleDC(hScreenDC);
    HBITMAP hBitmap = CreateCompatibleBitmap(hScreenDC, w, h);
    HBITMAP hOldBitmap = (HBITMAP)SelectObject(hMemoryDC, hBitmap);

    BitBlt(hMemoryDC, 0, 0, w, h, hScreenDC, geometry.x(), geometry.y(),
           SRCCOPY);

    Bitmap *gdiBitmap = Bitmap::FromHBITMAP(hBitmap, NULL);

    BitmapData bitmapData;
    Rect rect(0, 0, w, h);

    if (gdiBitmap->LockBits(&rect, ImageLockModeRead, PixelFormat32bppARGB,
                            &bitmapData) == Ok) {

      QImage qtImage(static_cast<uchar *>(bitmapData.Scan0), w, h,
                     bitmapData.Stride, QImage::Format_ARGB32);

      QImage safeImage = qtImage.copy();

      CapturedFrame frame;
      frame.image = safeImage;
      frame.geometry = geometry;
      frame.index = data->indexCounter++;
      frame.name = QString::fromWCharArray(mi.szDevice);

      UINT dpiX, dpiY;
      HRESULT hr = GetDpiForMonitor(hMonitor, MDT_EFFECTIVE_DPI, &dpiX, &dpiY);
      if (SUCCEEDED(hr)) {
        frame.devicePixelRatio = static_cast<qreal>(dpiX) / 96.0;
      } else {
        frame.devicePixelRatio = 1.0;
      }

      frame.image.setDevicePixelRatio(frame.devicePixelRatio);

      data->frames.push_back(frame);

      gdiBitmap->UnlockBits(&bitmapData);
    }

    delete gdiBitmap;
    SelectObject(hMemoryDC, hOldBitmap);
    DeleteObject(hBitmap);
    DeleteDC(hMemoryDC);
    ReleaseDC(NULL, hScreenDC);

    return TRUE;
  }
};

extern "C" ScreenGrabber *createWindowsEngine(QObject *parent) {
  return new ScreenGrabberWin(parent);
}
