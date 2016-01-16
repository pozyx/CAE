namespace Pozyx.CAE.Lib.Portable.CellSpaces
{
    // TODO: use with:
    // 1. .NET CPU runner (for validation and reference)
    //    - SingleThreadCpuRunner 
    //    - ideally some fast CPU runner - modification required (last time I could not make it work with BitArrayCellSpace, so I don't know)
    // 2. C++ CPU runner (for validation and reference)
    // 3. C++ GPU runner (limit concurrency to sizeof(int))
    // 4. tiled gpu runner (create padded cellspace variation)
    public class PackedIntArrayCellSpace
    {
        // TODO:
    }
}
