/*---------------------------------------------------------------------------+
|   EasyMesh - A Two-Dimensional Quality Mesh Generator                      |
|                                                                            |
|   Copyright (C) 2008 Bojan Niceno - bojan.niceno@psi.ch                    |
|                                                                            |
|   ParaView VTK legacy ASCII output (.vtk)                                  |
|   Format: VTK Unstructured Grid (version 2.0)                              |
|                                                                            |
|   The file can be loaded directly into ParaView or VisIt via the          |
|   "VTK Legacy" reader.                                                     |
|                                                                            |
|   Cell type 5 = VTK_TRIANGLE (three-node linear triangle)                  |
|   Point data : BoundaryMarker (integer scalar field on nodes)              |
|   Cell  data : MaterialMarker (integer scalar field on elements)           |
+---------------------------------------------------------------------------*/
#include <cstdio>
#include <cstring>
#include "easymesh.h"

using namespace std;

/*==========================================================================*/
int EasyMesh::save_vtk()
{
 int  e, n;
 int  r_Nn = 0, r_Ne = 0;

 /*------------------------------------------------------------------+
 |  Count active (non-OFF, renumbered) nodes and elements            |
 +------------------------------------------------------------------*/
 for(n=0; n<node.size(); n++)
   if(node[n].mark != OFF && node[n].new_numb != OFF)
     r_Nn++;

 for(e=0; e<elem.size(); e++)
   if(elem[e].mark != OFF && elem[e].new_numb != OFF)
     r_Ne++;

 /*------------------------------------------------------------------+
 |  Build temporary renumbered node table                            |
 +------------------------------------------------------------------*/
 Node r_node(r_Nn);
 r_node.increase_size(r_Nn);

 for(n=0; n<node.size(); n++)
   if(node[n].mark != OFF && node[n].new_numb != OFF)
    {
     r_node[node[n].new_numb].x    = node[n].x;
     r_node[node[n].new_numb].y    = node[n].y;
     r_node[node[n].new_numb].mark = node[n].mark;
    }

 /*------------------------------------------------------------------+
 |  Build temporary renumbered element table                         |
 +------------------------------------------------------------------*/
 Element r_elem(r_Ne);
 r_elem.increase_size(r_Ne);

 for(e=0; e<elem.size(); e++)
   if(elem[e].mark != OFF && elem[e].new_numb != OFF)
    {
     r_elem[elem[e].new_numb].i        = node[elem[e].i].new_numb;
     r_elem[elem[e].new_numb].j        = node[elem[e].j].new_numb;
     r_elem[elem[e].new_numb].k        = node[elem[e].k].new_numb;
     r_elem[elem[e].new_numb].material = elem[e].material;
    }

 /*------------------------------------------------------------------+
 |  Open output file: NAME.vtk                                       |
 |  After save() returns, name[len-1] holds the last extension char  |
 |  ('s'). We reuse the same buffer pattern to write our extension.  |
 +------------------------------------------------------------------*/
 char vtk_name[84];
 strncpy(vtk_name, name, 80);
 vtk_name[80] = '\0';
 /* name[len-1] was last set to 's' by save(). Replace extension.   */
 vtk_name[len-2] = '\0';          /* strip ".s" -> base name        */
 strcat(vtk_name, ".vtk");

 FILE *out;
 if((out = fopen(vtk_name, "w")) == NULL)
  {
   fprintf(stderr, "Cannot save VTK file %s !\n", vtk_name);
   return 1;
  }

 /*------------------------------------------------------------------+
 |  VTK legacy file header (must be exactly these lines)            |
 +------------------------------------------------------------------*/
 fprintf(out, "# vtk DataFile Version 2.0\n");
 fprintf(out, "EasyMesh triangulation\n");
 fprintf(out, "ASCII\n");
 fprintf(out, "DATASET UNSTRUCTURED_GRID\n");

 /*------------------------------------------------------------------+
 |  Points section                                                   |
 |  VTK requires 3D coordinates; z = 0 for 2D meshes                |
 +------------------------------------------------------------------*/
 fprintf(out, "POINTS %d float\n", r_Nn);
 for(n=0; n<r_Nn; n++)
   fprintf(out, " %18.15e  %18.15e  0.0\n",
           r_node[n].x, r_node[n].y);

 /*------------------------------------------------------------------+
 |  Cells section                                                    |
 |  Each triangle entry: 3 (node count)  i  j  k (0-based)         |
 |  Total integers = r_Ne * 4                                        |
 +------------------------------------------------------------------*/
 fprintf(out, "CELLS %d %d\n", r_Ne, r_Ne * 4);
 for(e=0; e<r_Ne; e++)
   fprintf(out, " 3  %d  %d  %d\n",
           r_elem[e].i,
           r_elem[e].j,
           r_elem[e].k);

 /*------------------------------------------------------------------+
 |  Cell types: 5 = VTK_TRIANGLE                                    |
 +------------------------------------------------------------------*/
 fprintf(out, "CELL_TYPES %d\n", r_Ne);
 for(e=0; e<r_Ne; e++)
   fprintf(out, " 5\n");

 /*------------------------------------------------------------------+
 |  Point data: BoundaryMarker                                       |
 +------------------------------------------------------------------*/
 fprintf(out, "POINT_DATA %d\n", r_Nn);
 fprintf(out, "SCALARS BoundaryMarker int 1\n");
 fprintf(out, "LOOKUP_TABLE default\n");
 for(n=0; n<r_Nn; n++)
   fprintf(out, " %d\n", r_node[n].mark);

 /*------------------------------------------------------------------+
 |  Cell data: MaterialMarker                                        |
 +------------------------------------------------------------------*/
 fprintf(out, "CELL_DATA %d\n", r_Ne);
 fprintf(out, "SCALARS MaterialMarker int 1\n");
 fprintf(out, "LOOKUP_TABLE default\n");
 for(e=0; e<r_Ne; e++)
   fprintf(out, " %d\n", r_elem[e].material);

 fflush(out);
 fclose(out);

 return 0;
}
