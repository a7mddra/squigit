/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include "ScreenGrabber.h"
#include <QGuiApplication>
#include <QScreen>
#include <iostream>

#include <windows.h>
#include <gdiplus.h>

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
  ScreenGrabberWin(QObject *parent = nullptr)
      : ScreenGrabber(parent), m_gdiplusToken(0), m_gdiplusReady(false) {
    GdiplusStartupInput gdiplusStartupInput;
    Status status =
        GdiplusStartup(&m_gdiplusToken, &gdiplusStartupInput, nullptr);
    m_gdiplusReady = (status == Ok);
    if (!m_gdiplusReady) {
      std::cerr << "CAPTURE_NATIVE_ERROR: GdiplusStartup failed with status "
                << static_cast<int>(status) << std::endl;
    }
  }

  ~ScreenGrabberWin() override {
    if (m_gdiplusReady && m_gdiplusToken != 0) {
      GdiplusShutdown(m_gdiplusToken);
    }
  }

  std::vector<CapturedFrame> captureAll() override {
    if (!m_gdiplusReady) {
      return {};
    }

    MonitorData data;

    HDC hdc = GetDC(nullptr);
    if (!hdc) {
      std::cerr << "CAPTURE_NATIVE_ERROR: GetDC returned null in captureAll"
                << std::endl;
      return {};
    }

    EnumDisplayMonitors(hdc, nullptr, MonitorEnumProc,
                        reinterpret_cast<LPARAM>(&data));
    ReleaseDC(nullptr, hdc);

    ScreenGrabber::sortLeftToRight(data.frames);

    for (size_t i = 0; i < data.frames.size(); i++) {
      data.frames[i].index = static_cast<int>(i);
    }

    return data.frames;
  }

private:
  ULONG_PTR m_gdiplusToken;
  bool m_gdiplusReady;

  static BOOL CALLBACK MonitorEnumProc(HMONITOR hMonitor, HDC hdcMonitor,
                                       LPRECT lprcMonitor, LPARAM dwData) {
    (void)hdcMonitor;
    (void)lprcMonitor;

    MonitorData *data = reinterpret_cast<MonitorData *>(dwData);
    if (!data) {
      return TRUE;
    }

    MONITORINFOEXW mi;
    mi.cbSize = sizeof(MONITORINFOEXW);
    if (!GetMonitorInfoW(hMonitor, &mi))
      return TRUE;

    QRect geometry(mi.rcMonitor.left, mi.rcMonitor.top,
                   mi.rcMonitor.right - mi.rcMonitor.left,
                   mi.rcMonitor.bottom - mi.rcMonitor.top);

    int w = geometry.width();
    int h = geometry.height();
    if (w <= 0 || h <= 0) {
      return TRUE;
    }

    HDC hScreenDC = GetDC(nullptr);
    if (!hScreenDC) {
      return TRUE;
    }

    HDC hMemoryDC = CreateCompatibleDC(hScreenDC);
    if (!hMemoryDC) {
      ReleaseDC(nullptr, hScreenDC);
      return TRUE;
    }

    HBITMAP hBitmap = CreateCompatibleBitmap(hScreenDC, w, h);
    if (!hBitmap) {
      DeleteDC(hMemoryDC);
      ReleaseDC(nullptr, hScreenDC);
      return TRUE;
    }

    HGDIOBJ hOldBitmap = SelectObject(hMemoryDC, hBitmap);
    if (!hOldBitmap || hOldBitmap == HGDI_ERROR) {
      DeleteObject(hBitmap);
      DeleteDC(hMemoryDC);
      ReleaseDC(nullptr, hScreenDC);
      return TRUE;
    }

    if (!BitBlt(hMemoryDC, 0, 0, w, h, hScreenDC, geometry.x(), geometry.y(),
                SRCCOPY)) {
      SelectObject(hMemoryDC, hOldBitmap);
      DeleteObject(hBitmap);
      DeleteDC(hMemoryDC);
      ReleaseDC(nullptr, hScreenDC);
      return TRUE;
    }

    Bitmap *gdiBitmap = Bitmap::FromHBITMAP(hBitmap, nullptr);
    if (!gdiBitmap || gdiBitmap->GetLastStatus() != Ok) {
      delete gdiBitmap;
      SelectObject(hMemoryDC, hOldBitmap);
      DeleteObject(hBitmap);
      DeleteDC(hMemoryDC);
      ReleaseDC(nullptr, hScreenDC);
      return TRUE;
    }

    BitmapData bitmapData = {};
    Rect rect(0, 0, w, h);

    Status lockStatus = gdiBitmap->LockBits(&rect, ImageLockModeRead,
                                            PixelFormat32bppARGB, &bitmapData);
    if (lockStatus == Ok) {

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
    ReleaseDC(nullptr, hScreenDC);

    return TRUE;
  }
};

extern "C" ScreenGrabber *createWindowsEngine(QObject *parent) {
  return new ScreenGrabberWin(parent);
}
