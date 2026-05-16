import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/widgets.dart';

import 'package:flutter_hbb/common.dart';

Future<ui.Image?> decodeImageFromPixels(
  Uint8List pixels,
  int width,
  int height,
  ui.PixelFormat format, {
  int? rowBytes,
  int? targetWidth,
  int? targetHeight,
  bool allowUpscaling = true,
}) async {
  if (targetWidth != null) {
    assert(allowUpscaling || targetWidth <= width);
    if (!(allowUpscaling || targetWidth <= width)) {
      print("not allow upscaling but targetWidth > width");
      return null;
    }
  }
  if (targetHeight != null) {
    assert(allowUpscaling || targetHeight <= height);
    if (!(allowUpscaling || targetHeight <= height)) {
      print("not allow upscaling but targetHeight > height");
      return null;
    }
  }

  final ui.ImmutableBuffer buffer;
  try {
    buffer = await ui.ImmutableBuffer.fromUint8List(pixels);
  } catch (e) {
    return null;
  }

  final ui.ImageDescriptor descriptor;
  try {
    descriptor = ui.ImageDescriptor.raw(
      buffer,
      width: width,
      height: height,
      rowBytes: rowBytes,
      pixelFormat: format,
    );
    if (!allowUpscaling) {
      if (targetWidth != null && targetWidth > descriptor.width) {
        targetWidth = descriptor.width;
      }
      if (targetHeight != null && targetHeight > descriptor.height) {
        targetHeight = descriptor.height;
      }
    }
  } catch (e) {
    print("ImageDescriptor.raw failed: $e");
    buffer.dispose();
    return null;
  }

  final ui.Codec codec;
  try {
    codec = await descriptor.instantiateCodec(
      targetWidth: targetWidth,
      targetHeight: targetHeight,
    );
  } catch (e) {
    print("instantiateCodec failed: $e");
    buffer.dispose();
    descriptor.dispose();
    return null;
  }

  final ui.FrameInfo frameInfo;
  try {
    frameInfo = await codec.getNextFrame();
  } catch (e) {
    print("getNextFrame failed: $e");
    codec.dispose();
    buffer.dispose();
    descriptor.dispose();
    return null;
  }

  codec.dispose();
  buffer.dispose();
  descriptor.dispose();
  return frameInfo.image;
}

class ImagePainter extends CustomPainter {
  ImagePainter({
    required this.image,
    required this.x,
    required this.y,
    required this.scale,
    this.logicalSize,
  });

  ui.Image? image;
  double x;
  double y;
  double scale;
  Size? logicalSize;

  @override
  void paint(Canvas canvas, Size size) {
    if (image == null) return;
    if (x.isNaN || y.isNaN) return;
    var scaleX = scale;
    var scaleY = scale;
    var drawX = x;
    var drawY = y;
    final logicalSize = this.logicalSize;
    if (logicalSize != null && image!.width > 0 && image!.height > 0) {
      final imageScaleX = logicalSize.width / image!.width;
      final imageScaleY = logicalSize.height / image!.height;
      if (imageScaleX.isFinite &&
          imageScaleY.isFinite &&
          imageScaleX > 0 &&
          imageScaleY > 0) {
        scaleX *= imageScaleX;
        scaleY *= imageScaleY;
        drawX /= imageScaleX;
        drawY /= imageScaleY;
      }
    }
    canvas.scale(scaleX, scaleY);
    // https://github.com/flutter/flutter/issues/76187#issuecomment-784628161
    // https://api.flutter-io.cn/flutter/dart-ui/FilterQuality.html
    var paint = Paint();
    final maxScale = scaleX > scaleY ? scaleX : scaleY;
    if ((maxScale - 1.0).abs() > 0.001) {
      paint.filterQuality = FilterQuality.medium;
      if (maxScale > 10.00000) {
        paint.filterQuality = FilterQuality.high;
      }
    }
    // It's strange that if (scale < 0.5 && paint.filterQuality == FilterQuality.medium)
    // The canvas.drawImage will not work on web
    if (isWeb) {
      paint.filterQuality = FilterQuality.high;
    }
    canvas.drawImage(
        image!,
        Offset(drawX.toInt().toDouble(), drawY.toInt().toDouble()),
        paint);
  }

  @override
  bool shouldRepaint(CustomPainter oldDelegate) {
    return oldDelegate != this;
  }
}
