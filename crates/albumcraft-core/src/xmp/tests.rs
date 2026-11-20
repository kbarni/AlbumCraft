use super::*;

/// Properties written as attributes, a hierarchy, a develop setting AlbumCraft doesn't know.
const ATTRS: &str = r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Some Toolkit 1.0">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:crs="http://ns.adobe.com/camera-raw-settings/1.0/"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:lr="http://ns.adobe.com/lightroom/1.0/"
    xmlns:exif="http://ns.adobe.com/exif/1.0/"
    xmp:Rating="4"
    crs:Exposure2012="+0.35"
    exif:GPSLatitude="44,50.5N"
    exif:GPSLongitude="0,34.75W">
   <dc:description>
    <rdf:Alt>
     <rdf:li xml:lang="fr">Le pont</rdf:li>
     <rdf:li xml:lang="x-default">The bridge &amp; the river</rdf:li>
    </rdf:Alt>
   </dc:description>
   <dc:subject>
    <rdf:Bag>
     <rdf:li>Bordeaux</rdf:li>
     <rdf:li>France</rdf:li>
     <rdf:li>bridge</rdf:li>
    </rdf:Bag>
   </dc:subject>
   <lr:hierarchicalSubject>
    <rdf:Bag>
     <rdf:li>Places|France|Bordeaux</rdf:li>
    </rdf:Bag>
   </lr:hierarchicalSubject>
   <crs:ToneCurve>
    <rdf:Seq><rdf:li>0, 0</rdf:li><rdf:li>255, 255</rdf:li></rdf:Seq>
   </crs:ToneCurve>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

/// Face regions in both item forms, a slash-separated tag list, people.
const REGIONS: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:mwg-rs="http://www.metadataworkinggroup.com/schemas/regions/"
    xmlns:stArea="http://ns.adobe.com/xmp/sType/Area#" xmlns:digiKam="http://www.digikam.org/ns/1.0/"
    xmlns:Iptc4xmpExt="http://iptc.org/std/Iptc4xmpExt/2008-02-29/">
   <mwg-rs:Regions rdf:parseType="Resource">
    <mwg-rs:RegionList>
     <rdf:Bag>
      <rdf:li rdf:parseType="Resource">
       <mwg-rs:Name>Alice</mwg-rs:Name>
       <mwg-rs:Type>Face</mwg-rs:Type>
       <mwg-rs:Area stArea:x="0.5" stArea:y="0.5" stArea:w="0.1" stArea:h="0.1"/>
      </rdf:li>
      <rdf:li>
       <rdf:Description mwg-rs:Name="Bob" mwg-rs:Type="Face">
        <mwg-rs:Area stArea:x="0.2" stArea:y="0.3" stArea:w="0.1" stArea:h="0.1"/>
       </rdf:Description>
      </rdf:li>
      <rdf:li>
       <rdf:Description mwg-rs:Name="Eiffel Tower" mwg-rs:Type="Focus"/>
      </rdf:li>
     </rdf:Bag>
    </mwg-rs:RegionList>
   </mwg-rs:Regions>
   <digiKam:TagsList>
    <rdf:Seq>
     <rdf:li>Family/Cousins</rdf:li>
     <rdf:li>Holidays</rdf:li>
    </rdf:Seq>
   </digiKam:TagsList>
   <Iptc4xmpExt:PersonInImage>
    <rdf:Bag><rdf:li>Alice</rdf:li><rdf:li>Carol</rdf:li></rdf:Bag>
   </Iptc4xmpExt:PersonInImage>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn reads_attributes_alternatives_and_hierarchy() {
    let n = parse(ATTRS).unwrap();
    assert_eq!(n.rating, Some(4));
    assert_eq!(n.caption.as_deref(), Some("The bridge & the river"));
    // Bordeaux and France are levels of the hierarchy, bridge a tag of its own
    assert_eq!(n.tags, Some(vec![s(&["Places", "France", "Bordeaux"]), s(&["bridge"])]));
    let [lat, lon] = n.gps.unwrap();
    assert!((lat - 44.841_666).abs() < 1e-5 && (lon + 0.579_166).abs() < 1e-5, "{lat} {lon}");
    assert_eq!(n.people, None);
}

/// Lightroom: every level above a tag listed too, in both lists.
const LIGHTROOM: &str = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="XMP Core 6.0.0">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:lr="http://ns.adobe.com/lightroom/1.0/">
   <dc:subject><rdf:Bag>
    <rdf:li>France</rdf:li><rdf:li>Grand Est</rdf:li><rdf:li>Aube</rdf:li><rdf:li>Bateau</rdf:li>
    <rdf:li>Femme</rdf:li><rdf:li>Marie</rdf:li><rdf:li>Sport</rdf:li><rdf:li>Plongee</rdf:li>
   </rdf:Bag></dc:subject>
   <lr:hierarchicalSubject><rdf:Bag>
    <rdf:li>France</rdf:li><rdf:li>France|Grand Est</rdf:li><rdf:li>France|Grand Est|Aube</rdf:li>
    <rdf:li>Bateau</rdf:li><rdf:li>Femme</rdf:li><rdf:li>Femme|Marie</rdf:li>
    <rdf:li>sport</rdf:li><rdf:li>Sport|Plongee</rdf:li>
   </rdf:Bag></lr:hierarchicalSubject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;

#[test]
fn levels_listed_above_a_tag_are_not_tags() {
    let n = parse(LIGHTROOM).unwrap();
    assert_eq!(n.tags, Some(vec![s(&["Bateau"]), s(&["Femme", "Marie"]), s(&["France", "Grand Est", "Aube"]), s(&["Sport", "Plongee"])]));
}

#[test]
fn written_keywords_include_the_levels_above() {
    let notes = XmpNotes { tags: Some(vec![s(&["Sport", "Plongee"]), s(&["Bateau"])]), ..Default::default() };
    let packet = merge(None, &notes, false).unwrap();
    for k in ["<rdf:li>Sport</rdf:li>", "<rdf:li>Plongee</rdf:li>", "<rdf:li>Sport|Plongee</rdf:li>"] {
        assert!(packet.contains(k), "{k} missing in {packet}");
    }
    assert_eq!(parse(&packet).unwrap(), notes.normalized());
}

#[test]
fn reads_face_regions_and_slash_tags() {
    let n = parse(REGIONS).unwrap();
    assert_eq!(n.people, Some(s(&["Alice", "Bob", "Carol"])));
    assert_eq!(n.tags, Some(vec![s(&["Family", "Cousins"]), s(&["Holidays"])]));
    assert_eq!(n.rating, None);
}

#[test]
fn rejected_and_odd_ratings() {
    let p = |r: &str| parse(&format!(r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="{NS_RDF}"><rdf:Description xmlns:xmp="{NS_XMP}" xmp:Rating="{r}"/></rdf:RDF></x:xmpmeta>"#)).unwrap().rating;
    assert_eq!(p("-1"), Some(-1));
    assert_eq!(p("3.0"), Some(3));
    assert_eq!(p("99"), Some(5));
    assert_eq!(p("NaN"), None);
    assert_eq!(p("three"), None);
}

#[test]
fn merge_keeps_what_it_does_not_know() {
    let notes = XmpNotes {
        rating: Some(2),
        caption: Some("New <caption>".into()),
        tags: Some(vec![s(&["Places", "Spain"]), s(&["sea"])]),
        people: Some(s(&["Dora"])),
        flag: Some(Flag::Pick),
        ..Default::default()
    };
    let out = merge(Some(ATTRS), &notes, true).unwrap();
    assert!(out.contains("crs:Exposure2012=\"+0.35\""), "{out}");
    assert!(out.contains("<crs:ToneCurve>"), "{out}");
    // GPS not set by the notes: kept
    assert!(out.contains("exif:GPSLatitude=\"44,50.5N\""), "{out}");
    let back = parse(&out).unwrap();
    assert_eq!(back.rating, Some(2));
    assert_eq!(back.flag, Some(Flag::Pick));
    assert_eq!(back.caption.as_deref(), Some("New <caption>"));
    assert_eq!(back.tags, Some(vec![s(&["Places", "Spain"]), s(&["sea"])]));
    assert_eq!(back.people, Some(s(&["Dora"])));
    assert!(back.gps.is_some());
    // a second merge changes nothing more
    assert_eq!(merge(Some(&out), &notes, true).unwrap(), out);
}

#[test]
fn merge_removes_emptied_fields_and_gps() {
    let notes = XmpNotes { caption: Some(String::new()), tags: Some(vec![]), gps_removed: true, ..Default::default() };
    let out = merge(Some(ATTRS), &notes, true).unwrap();
    let back = parse(&out).unwrap();
    assert_eq!(back.caption, None);
    assert_eq!(back.tags, None);
    assert_eq!(back.gps, None);
    assert_eq!(back.rating, Some(4));
}

#[test]
fn merge_keeps_regions_and_rewrites_slash_tags() {
    let notes = XmpNotes { tags: Some(vec![s(&["Family", "Aunts"])]), people: Some(s(&["Alice"])), ..Default::default() };
    let out = merge(Some(REGIONS), &notes, true).unwrap();
    assert!(out.contains("<mwg-rs:Regions"), "{out}");
    assert!(out.contains("<rdf:li>Family/Aunts</rdf:li>"), "{out}");
    let back = parse(&out).unwrap();
    assert_eq!(back.tags, Some(vec![s(&["Family", "Aunts"])]));
    // the regions still name Bob
    assert_eq!(back.people, Some(s(&["Alice", "Bob"])));
}

#[test]
fn edits_round_trip() {
    let ops = vec![EditOp::Rotate(1)];
    let out = merge(None, &XmpNotes { edits: Some(ops.clone()), ..Default::default() }, false).unwrap();
    assert!(out.starts_with("<?xpacket begin"));
    assert_eq!(parse(&out).unwrap().edits, Some(ops));
    // a later format is not read
    let later = out.replace("\"format\":1", "\"format\":9");
    assert_ne!(later, out);
    assert_eq!(parse(&later).unwrap().edits, None);
}

#[test]
fn hostile_packets_dont_panic() {
    for bad in [
        "",
        "<",
        "not xml at all",
        "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"></x:xmpmeta>",
        "<!DOCTYPE x [<!ENTITY a \"aaaa\">]><x>&a;</x>",
        "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description><dc:subject xmlns:dc=\"http://purl.org/dc/elements/1.1/\"><rdf:Bag><rdf:li>",
        "\u{feff}\0\0\0",
    ] {
        let _ = parse(bad);
        let _ = merge(Some(bad), &XmpNotes { rating: Some(1), ..Default::default() }, true);
    }
    assert!(merge(Some("<broken"), &XmpNotes::default(), true).is_err());
}

/// A small real JPEG with an EXIF segment and an XMP segment.
fn jpeg_with_packet(packet: &str) -> Vec<u8> {
    let img = image::RgbImage::from_pixel(8, 8, image::Rgb([200, 100, 50]));
    let mut jpeg = vec![];
    img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new(&mut jpeg)).unwrap();
    jpeg_with_xmp(&jpeg, packet).unwrap()
}

fn after_sos(j: &[u8]) -> &[u8] {
    let i = j.windows(2).position(|w| w == [0xff, 0xda]).unwrap();
    &j[i..]
}

#[test]
fn jpeg_packet_replaced_and_picture_data_kept() {
    let dir = tempdir("jpeg");
    let pic = dir.join("a.jpg");
    let file = jpeg_with_packet(ATTRS);
    std::fs::write(&pic, &file).unwrap();
    let found = read_for(&pic, None);
    assert!(found.embedded);
    assert_eq!(found.notes.unwrap().rating, Some(4));

    let w = write_for(&pic, None, &XmpNotes { rating: Some(1), ..Default::default() }, true).unwrap();
    assert_eq!(w, Written::Embedded);
    let new = std::fs::read(&pic).unwrap();
    assert_eq!(after_sos(&new), after_sos(&file));
    let back = read_for(&pic, None).notes.unwrap();
    assert_eq!(back.rating, Some(1));
    assert_eq!(back.caption.as_deref(), Some("The bridge & the river"));
    assert!(image::load_from_memory(&new).is_ok());
    assert!(std::fs::read_dir(&dir).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".craft_tmp")), "no temporary file left");
}

#[test]
fn sidecar_when_embedding_is_off_or_impossible() {
    let dir = tempdir("side");
    let pic = dir.join("b.jpg");
    let file = jpeg_with_packet(ATTRS);
    std::fs::write(&pic, &file).unwrap();
    let w = write_for(&pic, None, &XmpNotes { rating: Some(5), ..Default::default() }, false).unwrap();
    let side = dir.join("b.jpg.xmp");
    assert_eq!(w, Written::Sidecar(side.clone()));
    assert_eq!(std::fs::read(&pic).unwrap(), file, "the picture is untouched");
    // the sidecar now wins
    let s = sidecar_of(&pic, &|p| p.exists()).unwrap();
    assert_eq!(read_for(&pic, Some(&s)).notes.unwrap().rating, Some(5));
    // and is merged into the next time
    write_for(&pic, Some(&s), &XmpNotes { caption: Some("x".into()), ..Default::default() }, true).unwrap();
    let back = read_for(&pic, Some(&s)).notes.unwrap();
    assert_eq!((back.rating, back.caption.as_deref()), (Some(5), Some("x")));

    // a picture without XMP inside gets a sidecar too
    let plain = dir.join("c.png");
    image::RgbImage::new(4, 4).save(&plain).unwrap();
    let before = std::fs::read(&plain).unwrap();
    let w = write_for(&plain, None, &XmpNotes { rating: Some(3), ..Default::default() }, true).unwrap();
    assert_eq!(w, Written::Sidecar(dir.join("c.png.xmp")));
    assert_eq!(std::fs::read(&plain).unwrap(), before);
}

#[test]
fn broken_sidecar_is_not_replaced() {
    let dir = tempdir("broken");
    let pic = dir.join("d.jpg");
    std::fs::write(&pic, jpeg_with_packet(ATTRS)).unwrap();
    let side = dir.join("d.xmp");
    std::fs::write(&side, "<x:xmpmeta oops").unwrap();
    assert_eq!(sidecar_of(&pic, &|p| p.exists()), Some(side.clone()));
    assert!(write_for(&pic, Some(&side), &XmpNotes { rating: Some(1), ..Default::default() }, true).is_err());
    assert_eq!(std::fs::read_to_string(&side).unwrap(), "<x:xmpmeta oops");
}

#[test]
fn png_packet_read_and_replaced() {
    let img = image::RgbImage::from_pixel(4, 4, image::Rgb([1, 2, 3]));
    let mut png = std::io::Cursor::new(vec![]);
    img.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let png = png.into_inner();
    // put an XMP iTXt chunk after IHDR (8 + 25 bytes)
    let mut body = PNG_KEYWORD.to_vec();
    body.extend_from_slice(&[0, 0, 0, 0, 0]);
    body.extend_from_slice(ATTRS.as_bytes());
    let mut chunk = (body.len() as u32).to_be_bytes().to_vec();
    let mut typed = b"iTXt".to_vec();
    typed.extend_from_slice(&body);
    chunk.extend_from_slice(&typed);
    chunk.extend_from_slice(&crc32(&typed).to_be_bytes());
    let mut file = png[..33].to_vec();
    file.extend_from_slice(&chunk);
    file.extend_from_slice(&png[33..]);

    let dir = tempdir("png");
    let pic = dir.join("e.png");
    std::fs::write(&pic, &file).unwrap();
    assert_eq!(read_for(&pic, None).notes.unwrap().rating, Some(4));
    assert_eq!(write_for(&pic, None, &XmpNotes { rating: Some(2), ..Default::default() }, true).unwrap(), Written::Embedded);
    assert_eq!(read_for(&pic, None).notes.unwrap().rating, Some(2));
    let decoded = image::open(&pic).unwrap().to_rgb8();
    assert_eq!(decoded, img);
}

#[test]
fn crc_matches_png() {
    // the CRC of the empty IEND chunk
    assert_eq!(crc32(b"IEND"), 0xae42_6082);
}

#[test]
fn damaged_jpeg_is_refused() {
    let file = jpeg_with_packet(ATTRS);
    for cut in [3, 10, 40, file.len() / 2] {
        let _ = replace_embedded(&file[..cut], "<x/>");
        let _ = embedded_of(&craft_metadata::embedded(&file[..cut]));
    }
    assert!(replace_embedded(b"GIF89a", "<x/>").is_err());
    let big = "x".repeat(70_000);
    assert!(replace_embedded(&file, &big).is_err());
}

fn tempdir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("albumcraft-xmp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}


#[test]
fn dates_round_trip_with_their_precision() {
    let d = |t: &str| Some(crate::taken::parse(t).unwrap());
    for date in ["1978", "1978-07", "1978-07-14T10:30:00"] {
        let out = merge(None, &XmpNotes { date: d(date), ..Default::default() }, true).unwrap();
        assert_eq!(parse(&out).unwrap().date.map(|x| x.to_iso()).as_deref(), Some(date), "{out}");
        // a year or a month alone is not an EXIF date
        assert_eq!(out.contains("exif:DateTimeOriginal"), date.len() > 7, "{out}");
    }
    // a camera date written by another program is replaced, then removed
    let theirs = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:exif="http://ns.adobe.com/exif/1.0/" xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/" exif:DateTimeOriginal="2001-01-01T00:00:00" photoshop:DateCreated="2001-01-01T00:00:00" xmp:CreateDate="2020-05-05T10:00:00" xmp:Rating="2"/></rdf:RDF></x:xmpmeta>"#;
    let set = merge(Some(theirs), &XmpNotes { date: d("1978-07"), ..Default::default() }, true).unwrap();
    let back = parse(&set).unwrap();
    assert_eq!((back.date.map(|x| x.to_iso()), back.rating), (Some("1978-07".to_string()), Some(2)));
    assert!(!set.contains("2001-01-01"), "{set}");
    let gone = merge(Some(&set), &XmpNotes { date_removed: true, ..Default::default() }, true).unwrap();
    // xmp:CreateDate (a scanning program's day of the scan) is neither read nor removed
    assert_eq!(parse(&gone).unwrap().date, None);
    assert!(gone.contains("2020-05-05"), "{gone}");
}

#[test]
fn the_files_own_date_is_not_a_date_set() {
    let dir = tempdir("dates");
    let pic = dir.join("e.jpg");
    crate::exifinfo::write_demo_jpeg(&pic).unwrap();
    let camera = crate::filemeta::exif(&pic).and_then(|m| m.capture_time).unwrap();
    let side = dir.join("e.jpg.xmp");
    // a sidecar repeating the camera's date (as raw developers write it): none set
    let same = merge(None, &XmpNotes { date: Some(camera), rating: Some(1), ..Default::default() }, true).unwrap();
    std::fs::write(&side, same).unwrap();
    assert_eq!(read_for(&pic, Some(&side)).notes.unwrap().date, None);
    // another date: set
    let other = merge(None, &XmpNotes { date: Some(crate::taken::parse("1950").unwrap()), ..Default::default() }, true).unwrap();
    std::fs::write(&side, other).unwrap();
    assert_eq!(read_for(&pic, Some(&side)).notes.unwrap().date.map(|d| d.to_iso()).as_deref(), Some("1950"));
    let _ = std::fs::remove_dir_all(&dir);
}
