using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    public interface ICellSpace
    {        
        int Offset { get; }        
        int Length { get; }
        void Initialize(BitArray bitArray, int offset);        
        bool Get(int index);
        void Set(int index, bool value);
    }
}