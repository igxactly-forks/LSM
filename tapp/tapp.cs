
using System;
using System.Collections.Generic;
using System.IO;

using Zumero.LSM;
//using Zumero.LSM.fs;

public static class hack
{
    public static string from_utf8(this Stream s)
    {
        // note the arbitrary choice of getting this function from cs instead of fs
        // maybe utils should move into LSM_base
        return Zumero.LSM.fs.utils.ReadAll (s).FromUTF8 ();
    }

    public static void Insert(this IWrite w, byte[] k, byte[] v)
    {
        w.Insert (k, new MemoryStream(v) );
    }

    public static void Insert(this IWrite w, string k, byte[] v)
    {
        w.Insert (k.ToUTF8 (), new MemoryStream(v) );
    }

    public static void Insert(this IWrite w, string k, string v)
    {
        w.Insert (k.ToUTF8 (), new MemoryStream(v.ToUTF8 ()) );
    }

    public static void Delete(this IWrite w, string k)
    {
        w.Delete (k.ToUTF8 ());
    }

    public static void Seek(this ICursor csr, string k, SeekOp sop)
    {
        csr.Seek (k.ToUTF8(), sop);
    }

    public static byte[] ToUTF8(this string s)
    {
        return System.Text.Encoding.UTF8.GetBytes (s);
    }

    public static string FromUTF8(this byte[] ba)
    {
        return System.Text.Encoding.UTF8.GetString (ba, 0, ba.Length);
    }
}

public class foo
{
	private const int PAGE_SIZE = 256;

	private static int lastPage(Stream fs)
	{
		return (int)(fs.Length / PAGE_SIZE);
	}

    public static void Main(string[] argv)
    {
		var t1 = Zumero.LSM.cs.MemorySegment.Create();
		for (int i = 0; i < 10000; i++) {
			t1.Insert ((i * 2).ToString (), i.ToString ());
		}

		using (var fs = new FileStream("test_seek_ge_le_bigger", FileMode.Create)) {
			Zumero.LSM.cs.BTreeSegment.Create(fs, PAGE_SIZE, new Tuple<int,int>(1,0), t1.OpenCursor());

			{
				var csr = Zumero.LSM.cs.BTreeSegment.OpenCursor(fs, PAGE_SIZE, lastPage(fs));

				csr.Seek ("8088", SeekOp.SEEK_EQ);
				Console.WriteLine ("{0}", csr.IsValid ());
				Console.WriteLine ("{0}", csr.Key ().FromUTF8 ());
				//Assert.True (csr.IsValid ());

				csr.Seek ("8087", SeekOp.SEEK_EQ);
				Console.WriteLine ("{0}", csr.IsValid ());
				//Assert.False (csr.IsValid ());

				csr.Seek ("8087", SeekOp.SEEK_LE);
				Console.WriteLine ("{0}", csr.IsValid ());
				Console.WriteLine ("{0}", csr.Key ().FromUTF8 ());
				//Assert.True (csr.IsValid ());
				//Assert.Equal ("8086", csr.Key ().FromUTF8 ());

				csr.Seek ("8087", SeekOp.SEEK_GE);
				Console.WriteLine ("{0}", csr.IsValid ());
				Console.WriteLine ("{0}", csr.Key ().FromUTF8 ());
				//Assert.True (csr.IsValid ());
				//Assert.Equal ("8088", csr.Key ().FromUTF8 ());
			}
		}
    }
}
