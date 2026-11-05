// Each PNG given, rewritten without an alpha channel: App Store Connect
// refuses an App Store icon that has one, and Tauri's icon tool writes
// every iOS icon with one, its background composited but the channel kept.
// The icon is drawn onto white, which the iPhone's tile already is, so no
// pixel changes. Run after the iPhone's icons are made (docs/development.md,
// Icons); Apple's own frameworks, nothing to install. As an islands app on
// this stack redraws its own.
//
//   swift src-tauri/opaque.swift <png>...

import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

var failed = false
for path in CommandLine.arguments.dropFirst() {
  let url = URL(fileURLWithPath: path) as CFURL
  guard
    let source = CGImageSourceCreateWithURL(url, nil),
    let image = CGImageSourceCreateImageAtIndex(source, 0, nil),
    let space = CGColorSpace(name: CGColorSpace.sRGB),
    let context = CGContext(
      data: nil, width: image.width, height: image.height, bitsPerComponent: 8,
      bytesPerRow: 0, space: space,
      bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue)
  else {
    print("\(path): not read")
    failed = true
    continue
  }
  let whole = CGRect(x: 0, y: 0, width: image.width, height: image.height)
  context.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 1))
  context.fill(whole)
  context.draw(image, in: whole)
  guard
    let opaque = context.makeImage(),
    let out = CGImageDestinationCreateWithURL(url, UTType.png.identifier as CFString, 1, nil)
  else {
    print("\(path): not written")
    failed = true
    continue
  }
  CGImageDestinationAddImage(out, opaque, nil)
  if !CGImageDestinationFinalize(out) {
    print("\(path): not written")
    failed = true
  }
}
exit(failed ? 1 : 0)
