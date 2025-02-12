// These tests are based off the "Examples of XML Canonicalization" as
// specified in the w3c recommendation on the matter.
// https://www.w3.org/TR/xml-c14n/#Examples

use std::io::Cursor;

use tracing_test::traced_test;
use xml_c14n::Canonicalizer;

#[test]
#[traced_test]
fn example_3_1() {
    let input = r#"<?xml version="1.0"?>

<?xml-stylesheet   href="doc.xsl"
   type="text/xsl"   ?>

<!DOCTYPE doc SYSTEM "doc.dtd">

<doc>Hello, world!<!-- Comment 1 --></doc>

<?pi-without-data     ?>

<!-- Comment 2 -->

<!-- Comment 3 -->"#;

    let canonical_uncommented = r#"<?xml-stylesheet href="doc.xsl"
   type="text/xsl"   ?>
<doc>Hello, world!</doc>
<?pi-without-data?>"#;

    let canonical_commented = r#"<?xml-stylesheet href="doc.xsl"
   type="text/xsl"   ?>
<doc>Hello, world!<!-- Comment 1 --></doc>
<?pi-without-data?>
<!-- Comment 2 -->
<!-- Comment 3 -->"#;

    /*
     * Demonstrates:
     * · Loss of XML declaration
     * · Loss of DTD
     * · Normalization of whitespace outside of document element (first character of both canonical forms is '<'; single line breaks separate PIs and comments outside of document element)
     * · Loss of whitespace between PITarget and its data
     * · Retention of whitespace inside PI data
     * · Comment removal from uncommented canonical form, including delimiter for comments outside document element (the last character in both canonical forms is '>')
     */
    let mut result = vec![];
    Canonicalizer::read_from_str(input)
        .write_to_writer(Cursor::new(&mut result))
        .canonicalize(true)
        .expect("canonicalization shouldn't fail");
    assert_eq!(String::from_utf8_lossy(&result), canonical_commented);

    let mut result = vec![];
    Canonicalizer::read_from_str(input)
        .write_to_writer(Cursor::new(&mut result))
        .canonicalize(false)
        .expect("canonicalization shouldn't fail");
    assert_eq!(String::from_utf8_lossy(&result), canonical_uncommented);
}

#[test]
#[traced_test]
fn example_3_2() {
    let input = r#"<doc>
   <clean>   </clean>
   <dirty>   A   B   </dirty>
   <mixed>
      A
      <clean>   </clean>
      B
      <dirty>   A   B   </dirty>
      C
   </mixed>
</doc>"#;

    let canonical = r#"<doc>
   <clean>   </clean>
   <dirty>   A   B   </dirty>
   <mixed>
      A
      <clean>   </clean>
      B
      <dirty>   A   B   </dirty>
      C
   </mixed>
</doc>"#;

    /*
     * Demonstrates:
     * · Retain all whitespace between consecutive start tags, clean or dirty
     * · Retain all whitespace between consecutive end tags, clean or dirty
     * · Retain all whitespace between end tag/start tag pair, clean or dirty
     * · Retain all whitespace in character content, clean or dirty
     */
    let mut result = vec![];
    Canonicalizer::read_from_str(input)
        .write_to_writer(Cursor::new(&mut result))
        .canonicalize(true)
        .expect("canonicalization shouldn't fail");
    assert_eq!(String::from_utf8_lossy(&result), canonical);
}

#[test]
#[traced_test]
fn example_3_3() {
    let input = r#"<!DOCTYPE doc [<!ATTLIST e9 attr CDATA "default">]>
<doc>
   <e1   />
   <e2   ></e2>
   <e3   name = "elem3"   id="elem3"   />
   <e4   name="elem4"   id="elem4"   ></e4>
   <e5 a:attr="out" b:attr="sorted" attr2="all" attr="I'm"
      xmlns:b="http://www.ietf.org"
      xmlns:a="http://www.w3.org"
      xmlns="http://example.org"/>
   <e6 xmlns="" xmlns:a="http://www.w3.org">
      <e7 xmlns="http://www.ietf.org">
         <e8 xmlns="" xmlns:a="http://www.w3.org">
            <e9 xmlns="" xmlns:a="http://www.ietf.org"/>
         </e8>
      </e7>
   </e6>
</doc>"#;

    let canonical = r#"<doc>
   <e1></e1>
   <e2></e2>
   <e3 id="elem3" name="elem3"></e3>
   <e4 id="elem4" name="elem4"></e4>
   <e5 xmlns="http://example.org" xmlns:a="http://www.w3.org" xmlns:b="http://www.ietf.org" attr="I'm" attr2="all" b:attr="sorted" a:attr="out"></e5>
   <e6 xmlns:a="http://www.w3.org">
      <e7 xmlns="http://www.ietf.org">
         <e8 xmlns="">
            <e9 xmlns:a="http://www.ietf.org"></e9>
         </e8>
      </e7>
   </e6>
</doc>"#;

    /*
     * Demonstrates:
     * · Empty element conversion to start-end tag pair
     * · Normalization of whitespace in start and end tags
     * · Relative order of namespace and attribute axes
     * · Lexicographic ordering of namespace and attribute axes
     * · Retention of namespace prefixes from original document
     * · Elimination of superfluous namespace declarations
     */
    let mut result = vec![];
    Canonicalizer::read_from_str(input)
        .write_to_writer(Cursor::new(&mut result))
        .canonicalize(true)
        .expect("canonicalization shouldn't fail");
    assert_eq!(String::from_utf8_lossy(&result), canonical);
}

#[test]
#[traced_test]
fn example_3_4() {
    let input = r#"<!DOCTYPE doc [
<!ATTLIST normId id ID #IMPLIED>
<!ATTLIST normNames attr NMTOKENS #IMPLIED>
]>
<doc>
   <text>First line&#x0d;&#10;Second line</text>
   <value>&#x32;</value>
   <compute><![CDATA[value>"0" && value<"10" ?"valid":"error"]]></compute>
   <compute expr='value>"0" &amp;&amp; value&lt;"10" ?"valid":"error"'>valid</compute>
   <norm attr=' &apos;   &#x20;&#13;&#xa;&#9;   &apos; '/>
</doc>"#;

    let canonical = r#"<doc>
   <text>First line&#xD;
Second line</text>
   <value>2</value>
   <compute>value&gt;"0" &amp;&amp; value&lt;"10" ?"valid":"error"</compute>
   <compute expr="value>&quot;0&quot; &amp;&amp; value&lt;&quot;10&quot; ?&quot;valid&quot;:&quot;error&quot;">valid</compute>
   <norm attr=" '    &#xD;&#xA;&#x9;   ' "></norm>
</doc>"#;

    /*
     * Demonstrates:
     * · Character reference replacement
     * · Attribute value delimiters set to quotation marks (double quotes)
     * · Attribute value normalization
     * · CDATA section replacement
     * · Encoding of special characters as character references in attribute values (&amp;, &lt;, &quot;, &#xD;, &#xA;, &#x9;)
     * · Encoding of special characters as character references in text (&amp;, &lt;, &gt;, &#xD;)
     */
    let mut result = vec![];
    Canonicalizer::read_from_str(input)
        .write_to_writer(Cursor::new(&mut result))
        .canonicalize(true)
        .expect("canonicalization shouldn't fail");
    assert_eq!(String::from_utf8_lossy(&result), canonical);
}

// Example 3.5: Entity References
// This example is excluded as a feature from this crate.

#[test]
#[traced_test]
fn example_3_6() {
    let input = r#"<?xml version="1.0" encoding="ISO-8859-1"?>
<doc>&#169;</doc>"#;

    let canonical = r#"<doc>©</doc>"#;

    /*
     * Demonstrates:
     * · Effect of transcoding from a sample encoding to UTF-8
     */
    let mut result = vec![];
    Canonicalizer::read_from_str(input)
        .write_to_writer(Cursor::new(&mut result))
        .canonicalize(true)
        .expect("canonicalization shouldn't fail");
    assert_eq!(String::from_utf8_lossy(&result), canonical);
}

// Examples 3.7 and 3.8 use document subsets, which are not supported by this crate.
