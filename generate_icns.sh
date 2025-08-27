mkdir -p app.iconset
for s in 16 32 128 256 512; do
  s2=$((s*2))
  sips -z $s  $s  assets/icon.png --out app.iconset/icon_${s}x${s}.png
  sips -z $s2 $s2 assets/icon.png --out app.iconset/icon_${s}x${s}@2x.png
done
cp assets/icon.png app.iconset/icon_512x512@2x.png
iconutil -c icns app.iconset -o assets/icon.icns
file assets/icon.icns